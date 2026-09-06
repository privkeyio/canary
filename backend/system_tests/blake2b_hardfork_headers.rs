//! End to end check that Canary reads a post-hardfork block header off the wire.
//!
//! Bitcoin Knots' BLAKE2b proof-of-work hardfork made the block header self-describing: past the
//! activation height it may carry 84 bytes beyond the historical 80, and its block id is a BLAKE2b
//! digest rather than SHA256d. Stock rust-bitcoin parses 80 bytes and rejects the remainder, which
//! is the "data not consumed entirely when explicitly deserializing" failure that stops a sync.
//!
//! This drives the real `ElectrumClient` over a real socket against a fake Electrum server, so it
//! covers the whole chain: canary -> bdk_electrum's dependency electrum-client -> rust-bitcoin.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

use canary::electrum::ElectrumClient;

/// `profile_0_time_offset` from Bitcoin Knots' `src/test/data/block_header_v2.json` at tag
/// v29.4.1.knots20260508. 164 bytes.
const V2_HEADER_HEX: &str = "000000a01f1e1d1c1b1a191817161514131211100f0e0d0c0b0a0908070605040302010000112233445566778899aabbccddeeff00102030405060708090a0b0c0d0e0f0a8913577ffff001d0df0ad0b44332211efcdab89ffeeddccbbaa998877665544332211005802000003001c000000000000000000000000000000000040d10c008967452301efcdab8967452301efcdab8967452301efcdab8967452301efcdab";

/// The effective block time. The wire carries 1_999_999_400 with a `time_offset` of 600, so a
/// decoder that ignores the offset returns the wrong timestamp here.
const V2_EFFECTIVE_TIME: u64 = 2_000_000_000;

/// Mainnet block 100,000, to prove the legacy path is untouched. 80 bytes.
const V1_HEADER_HEX: &str = "0100000050120119172a610421a6c3011dd330d9df07b63616c2cc1f1cd00200000000006657a9252aacd5c0b2940996ecff952228c3067cc38d4885efb5a4ac4247e9f337221b4d4c86041b0f2b5710";
const V1_TIME: u64 = 1_293_623_863;

/// A minimal Electrum server that answers every `blockchain.block.header` with `header_hex`.
///
/// Returns the `tcp://` URL to point a client at.
fn fake_electrum_server(header_hex: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let reader = BufReader::new(stream.try_clone().expect("clone"));
            thread::spawn(move || {
                for line in reader.lines() {
                    let line = match line {
                        Ok(l) if !l.trim().is_empty() => l,
                        _ => break,
                    };
                    let req: serde_json::Value = match serde_json::from_str(&line) {
                        Ok(v) => v,
                        Err(_) => break,
                    };
                    let result = match req["method"].as_str().unwrap_or_default() {
                        "server.version" => serde_json::json!(["Fulcrum 1.12.0", "1.6"]),
                        "blockchain.block.header" => serde_json::json!(header_hex),
                        // The batch call bdk_electrum uses to build the chain tip. Protocol 1.6
                        // sends an array; the pre-1.6 form concatenates into one blob.
                        "blockchain.block.headers" => {
                            let count = req["params"][1].as_u64().unwrap_or(0) as usize;
                            serde_json::json!({
                                "count": count,
                                "max": 2016,
                                "headers": vec![header_hex; count],
                            })
                        }
                        _ => serde_json::Value::Null,
                    };
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": req["id"].clone(),
                        "result": result,
                    });
                    if writeln!(stream, "{}", response).is_err() {
                        break;
                    }
                    let _ = stream.flush();
                }
            });
        }
    });

    format!("tcp://{}", addr)
}

#[tokio::test]
async fn reads_an_extended_header_off_the_wire() {
    let url = fake_electrum_server(V2_HEADER_HEX);
    let client = ElectrumClient::new(&url).expect("connect to the fake server");

    let header = client
        .get_block_header(961_640)
        .await
        .expect("a 164 byte header must parse; this is the sync failure being fixed");

    assert_eq!(header.height, 961_640);
    // Proves the wire time and `time_offset` were recombined, not just that parsing succeeded.
    assert_eq!(header.timestamp, V2_EFFECTIVE_TIME);
}

#[tokio::test]
async fn still_reads_a_legacy_header_off_the_wire() {
    let url = fake_electrum_server(V1_HEADER_HEX);
    let client = ElectrumClient::new(&url).expect("connect to the fake server");

    let header = client
        .get_block_header(100_000)
        .await
        .expect("80 byte header");

    assert_eq!(header.height, 100_000);
    assert_eq!(header.timestamp, V1_TIME);
}

/// The chain tip path. `bdk_electrum::fetch_tip_and_latest_blocks` calls the batch
/// `blockchain.block.headers` on every sync and derives a `BlockHash` from each header, so this
/// covers the block id rather than just the timestamp.
#[tokio::test]
async fn batch_headers_yield_blake2b_block_ids() {
    use bdk_electrum::electrum_client::{self, ElectrumApi};

    let url = fake_electrum_server(V2_HEADER_HEX);
    let raw = electrum_client::Client::new(url.trim_start_matches("tcp://"))
        .expect("connect to the fake server");

    let res = raw
        .block_headers(961_640, 8)
        .expect("batch of extended headers must parse");
    assert_eq!(res.headers.len(), 8);

    // The BLAKE2b block id, not SHA256d over the first 80 bytes.
    for header in &res.headers {
        assert_eq!(
            header.block_hash().to_string(),
            "4b495dcf05d70a49785b799b22284fbcd9dd1209237c53c87e4674b15587d704"
        );
    }
}
