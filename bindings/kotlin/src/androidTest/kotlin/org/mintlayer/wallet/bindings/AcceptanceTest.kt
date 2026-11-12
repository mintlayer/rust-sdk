// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

package org.mintlayer.wallet.bindings

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * On-device acceptance tests: the public BIP39 test vector
 * ("abandon" x 23 + "art" — public vector, no secret material) must
 * reproduce, byte-for-byte, the outputs of the WASM reference
 * (`@mintlayer/wasm-lib` 1.4.0) through the generated Kotlin bindings.
 */
@RunWith(AndroidJUnit4::class)
class AcceptanceTest {

    private val mnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon abandon abandon art"

    private fun hex(bytes: ByteArray): String =
        bytes.joinToString("") { "%02x".format(it) }

    @Test
    fun derivesDefaultAccountKeyForTestnet() {
        val account = makeDefaultAccountPrivkey(mnemonic, Network.TESTNET, null)
        assertEquals(EXPECTED_XPRIV_TESTNET, hex(account))
    }

    @Test
    fun derivesDefaultAccountKeyForMainnet() {
        val account = makeDefaultAccountPrivkey(mnemonic, Network.MAINNET, null)
        assertEquals(EXPECTED_XPRIV_MAINNET, hex(account))
    }

    @Test
    fun derivesAddressChainForTestnet() {
        val account = makeDefaultAccountPrivkey(mnemonic, Network.TESTNET, null)
        val xpub = extendedPublicKeyFromExtendedPrivateKey(account)
        val pubkey = makeReceivingAddressPublicKey(xpub, 0u)
        assertEquals(
            "00035a6f28253f35e2ff1ab0b086b8c87f731159661157e5be2ca98188369625dc32",
            hex(pubkey),
        )
        val address = pubkeyToPubkeyhashAddress(pubkey, Network.TESTNET)
        assertEquals("tmt1qyxqfq37rra32sq6zhayqcpjwh4trc07mcfardwt", address)
    }

    @Test
    fun derivesAddressChainForMainnet() {
        val account = makeDefaultAccountPrivkey(mnemonic, Network.MAINNET, null)
        val xpub = extendedPublicKeyFromExtendedPrivateKey(account)
        val pubkey = makeReceivingAddressPublicKey(xpub, 0u)
        val address = pubkeyToPubkeyhashAddress(pubkey, Network.MAINNET)
        assertEquals("mtc1q82twmwwxa3l2pe26gf0zgghu69jtez3qq2ajqyl", address)
    }

    @Test
    fun signsAndVerifiesAMessage() {
        val account = makeDefaultAccountPrivkey(mnemonic, Network.TESTNET, null)
        val private0 = makeReceivingAddress(account, 0u)
        val public0 = publicKeyFromPrivateKey(private0)
        val message = "mintlayer on-device acceptance".toByteArray()
        val signature = signMessageForSpending(private0, message)
        assertTrue(verifySignatureForSpending(public0, signature, message))
    }
    private companion object {
        // Reference outputs of `@mintlayer/wasm-lib` 1.4.0 for the public
        // BIP39 test vector (default account extended private key blobs).
        private const val EXPECTED_XPRIV_TESTNET =
            "00038000002c800000018000000012ed17c3e72223beb38ea98ec1317581f02458f02b0e" +
                "cf4abe02c6490eca170a900d967bc0c84b9c2da3b55f7db9f76cbdc0469176a76d0114" +
                "1af2a705b4353a"
        private const val EXPECTED_XPRIV_MAINNET =
            "00038000002c80004d4c800000005280d67624d70241021ee99da728121cb69e2356e777" +
                "6cbc360fc25dedc35d8c3ee4db2b274aa3fac7024333d45f916ac113cd63c47e9444d3" +
                "9d46827dce1a24"
    }

}
