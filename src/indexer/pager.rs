// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Cursor pagination helper for the indexer's keyset-paginated listings.

use std::future::Future;
use std::pin::Pin;

use crate::indexer::error::Error;
use crate::indexer::types::Page;

/// The future produced by a [`Pager`]'s page fetch.
pub type PagedFuture<T> = Pin<Box<dyn Future<Output = Result<Page<T>, Error>> + Send>>;

/// A stateful walk over a cursor-paginated indexer listing (pools, the
/// global transaction listing, the holders listings, and the order book).
///
/// Constructed through the `*_pager` methods on [`crate::indexer::Client`]
/// (for example [`crate::indexer::Client::coin_holders_pager`]). Each fetch
/// transparently follows the listing's `next_cursor`, and the walk stops
/// once the server sends `next_cursor: null` — including an order book page
/// that was [`truncated`](crate::indexer::OrderBook::truncated), whose
/// cursor cannot be continued.
///
/// Cursors are minted by the indexer; the pager only ever replays what the
/// server sent (or, via [`Pager::start_from`], a cursor a caller persisted
/// earlier). A failed fetch leaves the pager positioned at the same cursor,
/// so the walk may simply be retried.
///
/// [`Pager::next`] and [`Pager::next_page`] are two views over the same
/// walk and should not be mixed: a manual [`Pager::next_page`] discards any
/// not-yet-yielded items buffered by [`Pager::next`].
///
/// Page stability is only guaranteed once the indexer's scanner is fully
/// caught up: during catch-up or a reorg a cursor walk may skip or repeat
/// an entry.
pub struct Pager<T> {
    fetch: Box<dyn FnMut(Option<String>) -> PagedFuture<T> + Send>,
    next_cursor: Option<String>,
    pending: std::vec::IntoIter<T>,
    done: bool,
}

impl<T> Pager<T> {
    pub(crate) fn new(
        fetch: impl FnMut(Option<String>) -> PagedFuture<T> + Send + 'static,
    ) -> Self {
        Self {
            fetch: Box::new(fetch),
            next_cursor: None,
            pending: Vec::new().into_iter(),
            done: false,
        }
    }

    /// Resumes a walk from a cursor obtained earlier (typically persisted
    /// after a previous [`Pager`] run).
    #[must_use]
    pub fn start_from(mut self, cursor: impl Into<String>) -> Self {
        self.next_cursor = Some(cursor.into());
        self
    }

    /// The cursor the next page fetch resumes from; `None` at the start of
    /// a fresh walk and once the walk is exhausted.
    #[must_use]
    pub fn cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }

    /// Fetches the next page, or `None` once the listing is exhausted.
    /// Any items buffered by [`Pager::next`] but not yet yielded are
    /// discarded.
    pub async fn next_page(&mut self) -> Result<Option<Page<T>>, Error> {
        if self.done {
            return Ok(None);
        }
        self.pending = Vec::new().into_iter();
        let cursor = self.next_cursor.take();
        let Page { items, next_cursor } = (self.fetch)(cursor).await?;
        self.done = next_cursor.is_none();
        self.next_cursor = next_cursor;
        Ok(Some(Page {
            items,
            next_cursor: self.next_cursor.clone(),
        }))
    }

    /// Returns the next item, fetching pages as needed; `None` once the
    /// listing is exhausted. A fetch error is yielded as `Some(Err(..))`
    /// and leaves the pager positioned at the same cursor, so the walk can
    /// be retried.
    pub async fn next(&mut self) -> Option<Result<T, Error>> {
        loop {
            if let Some(item) = self.pending.next() {
                return Some(Ok(item));
            }
            match self.next_page().await {
                Ok(Some(page)) => self.pending = page.items.into_iter(),
                Ok(None) => return None,
                Err(error) => return Some(Err(error)),
            }
        }
    }
}
