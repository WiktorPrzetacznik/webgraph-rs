/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use anyhow::Result;
use clap::Parser;
use dsi_bitstream::prelude::*;
use itertools::Itertools;
use lender::*;
use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;
use std::hint::black_box;
use webgraph::graphs::bvgraph;
use webgraph::prelude::*;

#[derive(Parser, Debug)]
#[command(about = "Benchmarks the Rust Webgraph implementation", long_about = None)]
struct Args {
    /// The basename of the graph.
    basename: String,

    /// The number of test repetitions
    #[arg(short, long, default_value = "10")]
    repeats: usize,

    /// The number of successor lists in random-access tests
    #[arg(short, long, default_value = "1000000")]
    n: usize,

    /// Test sequential access speed by scanning the whole graph
    #[arg(short = 's', long)]
    sequential: bool,

    /// Test random access to the first successor
    #[arg(short = 'f', long)]
    first: bool,

    /// Test sequential degrees_only access speed by scanning the whole graph
    #[arg(short = 'd', long)]
    degrees_only: bool,

    /// Do not test speed, but check that the sequential and random-access successor lists are the same
    #[arg(short = 'c', long)]
    check: bool,
}

fn bench_webgraph<E: Endianness + 'static>(args: Args) -> Result<()>
where
    for<'a> BufBitReader<E, MemWordReader<u32, &'a [u32]>>: CodeRead<E> + BitSeek,
{
    if args.check {
        // Create a sequential reader
        let seq_graph = BVGraphSeq::with_basename("tests/data/cnr-2000")
            .endianness::<E>()
            .load()?;
        // create a random access reader;
        let random_reader = BVGraph::with_basename(&args.basename)
            .endianness::<E>()
            .load()?;

        // Check that sequential and random-access interfaces return the same result
        let mut seq_iter = seq_graph.iter();
        let mut deg_reader = seq_graph.iter_degrees();
        for node_id in 0..seq_graph.num_nodes() {
            let seq = seq_iter.next().unwrap();
            let random = random_reader.successors(node_id).collect::<Vec<_>>();

            assert_eq!(deg_reader.next_degree()?, seq.1.len(), "{}", node_id);
            assert_eq!(seq.1.collect_vec(), random, "{}", node_id);
        }
    } else if args.sequential {
        // Sequential speed testx
        for _ in 0..args.repeats {
            // Create a sequential reader
            let mut c: u64 = 0;

            let seq_graph = BVGraphSeq::with_basename(&args.basename)
                .endianness::<E>()
                .load()?;

            let start = std::time::Instant::now();
            let mut iter = seq_graph.iter();
            while let Some((_, succ)) = iter.next() {
                c += succ.count() as u64;
            }
            println!(
                "Sequential:{:>20} ns/arc",
                (start.elapsed().as_secs_f64() / c as f64) * 1e9
            );

            assert_eq!(c, seq_graph.num_arcs_hint().unwrap());
        }
    } else if args.degrees_only {
        // Sequential speed test
        for _ in 0..args.repeats {
            let seq_graph = BVGraphSeq::with_basename(&args.basename)
                .endianness::<E>()
                .load()?;
            let mut deg_reader = seq_graph.iter_degrees();

            let mut c: u64 = 0;
            let start = std::time::Instant::now();
            for _ in 0..seq_graph.num_nodes() {
                c += deg_reader.next_degree()? as u64;
            }
            println!(
                "Degrees Only:{:>20} ns/arc",
                (start.elapsed().as_secs_f64() / c as f64) * 1e9
            );

            assert_eq!(c, seq_graph.num_arcs_hint().unwrap());
        }
    } else {
        let graph = BVGraph::with_basename(&args.basename)
            .endianness::<E>()
            .load()?;
        // Random-access speed test
        for _ in 0..args.repeats {
            // create a random access reader;

            let mut random = SmallRng::seed_from_u64(0);
            let mut c: usize = 0;
            let mut u: usize = 0;

            let start = std::time::Instant::now();
            if args.first {
                for _ in 0..args.n {
                    u += graph
                        .successors(random.gen_range(0..graph.num_nodes()))
                        .next()
                        .unwrap_or(0);
                    c += 1;
                }
            } else {
                for _ in 0..args.n {
                    c += graph
                        .successors(random.gen_range(0..graph.num_nodes()))
                        .count();
                }
            }

            println!(
                "{}:    {:>20} ns/arc",
                if args.first { "First" } else { "Random" },
                (start.elapsed().as_secs_f64() / c as f64) * 1e9
            );
            black_box(u);
        }
    }
    Ok(())
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    stderrlog::new()
        .verbosity(2)
        .timestamp(stderrlog::Timestamp::Second)
        .init()?;

    match get_endianess(&args.basename)?.as_str() {
        #[cfg(any(
            feature = "be_bins",
            not(any(feature = "be_bins", feature = "le_bins"))
        ))]
        BE::NAME => bench_webgraph::<BE>(args),
        #[cfg(any(
            feature = "le_bins",
            not(any(feature = "be_bins", feature = "le_bins"))
        ))]
        LE::NAME => bench_webgraph::<LE>(args),
        e => panic!("Unknown endianness: {}", e),
    }
}
