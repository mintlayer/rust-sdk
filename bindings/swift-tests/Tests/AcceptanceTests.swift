// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

import XCTest
import MintlayerWalletBindings

/// On-simulator acceptance tests: the public BIP39 test vector
/// ("abandon" x 23 + "art" — public vector, no secret material) must
/// reproduce, byte-for-byte, the outputs of the WASM reference
/// (`@mintlayer/wasm-lib` 1.4.0) through the generated Swift bindings.
final class AcceptanceTests: XCTestCase {

    private let mnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon " +
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon " +
        "abandon abandon abandon abandon abandon art"

    private let expectedXprivTestnet =
        "00038000002c800000018000000012ed17c3e72223beb38ea98ec1317581f02458f02b0e" +
        "cf4abe02c6490eca170a900d967bc0c84b9c2da3b55f7db9f76cbdc0469176a76d01141a" +
        "f2a705b4353a"

    private let expectedXprivMainnet =
        "00038000002c80004d4c800000005280d67624d70241021ee99da728121cb69e2356e777" +
        "6cbc360fc25dedc35d8c3ee4db2b274aa3fac7024333d45f916ac113cd63c47e9444d39d" +
        "46827dce1a24"

    private func hex(_ data: Data) -> String {
        data.map { String(format: "%02x", $0) }.joined()
    }

    func testDerivesDefaultAccountKeyForTestnet() throws {
        let account = try makeDefaultAccountPrivkey(
            mnemonic: mnemonic, network: .testnet, passphrase: nil
        )
        XCTAssertEqual(hex(account), expectedXprivTestnet)
    }

    func testDerivesDefaultAccountKeyForMainnet() throws {
        let account = try makeDefaultAccountPrivkey(
            mnemonic: mnemonic, network: .mainnet, passphrase: nil
        )
        XCTAssertEqual(hex(account), expectedXprivMainnet)
    }

    func testDerivesAddressChainForTestnet() throws {
        let account = try makeDefaultAccountPrivkey(
            mnemonic: mnemonic, network: .testnet, passphrase: nil
        )
        let xpub = try extendedPublicKeyFromExtendedPrivateKey(accountPrivkey: account)
        let pubkey = try makeReceivingAddressPublicKey(accountPublicKey: xpub, keyIndex: 0)
        XCTAssertEqual(
            hex(pubkey),
            "00035a6f28253f35e2ff1ab0b086b8c87f731159661157e5be2ca98188369625dc32"
        )
        let address = try pubkeyToPubkeyhashAddress(publicKey: pubkey, network: .testnet)
        XCTAssertEqual(address, "tmt1qyxqfq37rra32sq6zhayqcpjwh4trc07mcfardwt")
    }

    func testDerivesAddressChainForMainnet() throws {
        let account = try makeDefaultAccountPrivkey(
            mnemonic: mnemonic, network: .mainnet, passphrase: nil
        )
        let address = try pubkeyToPubkeyhashAddress(
            publicKey: try makeReceivingAddressPublicKey(
                accountPublicKey: try extendedPublicKeyFromExtendedPrivateKey(
                    accountPrivkey: account
                ),
                keyIndex: 0
            ),
            network: .mainnet
        )
        XCTAssertEqual(address, "mtc1q82twmwwxa3l2pe26gf0zgghu69jtez3qq2ajqyl")
    }

    func testSignsAndVerifiesAMessage() throws {
        let account = try makeDefaultAccountPrivkey(
            mnemonic: mnemonic, network: .testnet, passphrase: nil
        )
        let private0 = try makeReceivingAddress(accountPrivkey: account, keyIndex: 0)
        let public0 = try publicKeyFromPrivateKey(privateKey: private0)
        let message = Data("mintlayer on-device acceptance".utf8)
        let signature = try signMessageForSpending(privateKey: private0, message: message)
        XCTAssertTrue(try verifySignatureForSpending(
            publicKey: public0, signature: signature, message: message
        ))
    }
}
