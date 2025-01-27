/*
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 * SPDX-FileCopyrightText: 2023 Tommaso Fontana
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::prelude::*;
use anyhow::{ensure, Result};
use clap::{ArgMatches, Args, Command, FromArgMatches};
use epserde::prelude::*;
use mmap_rs::MmapFlags;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use sux::traits::BitFieldSlice;

pub const COMMAND_NAME: &str = "comp";

#[derive(Args, Debug)]
#[command(about = "Compose multiple permutations into a single one", long_about = None)]
pub struct CliArgs {
    /// The path of the resulting permutation.
    pub dst: PathBuf,

    /// Filenames of the permutations to compose (in order of application).
    pub perms: Vec<PathBuf>,

    #[arg(short, long)]
    /// Save the permutation in ε-serde format.
    pub epserde: bool,
}

pub fn cli(command: Command) -> Command {
    command.subcommand(CliArgs::augment_args(Command::new(COMMAND_NAME)).display_order(0))
}

pub fn main(submatches: &ArgMatches) -> Result<()> {
    merge_perms(CliArgs::from_arg_matches(submatches)?)
}

pub fn merge_perms(args: CliArgs) -> Result<()> {
    let start = std::time::Instant::now();

    if args.epserde {
        let mut perm = Vec::new();
        for path in args.perms {
            let p = <Vec<usize>>::mmap(&path, Flags::RANDOM_ACCESS)?;
            perm.push(p);
        }
        let mut merged = Vec::new();

        ensure!(
            perm.iter().all(|p| p.len() == perm[0].len()),
            "All permutations must have the same length"
        );

        for i in 0..perm[0].len() {
            let mut v = i;
            for p in &perm {
                v = p[v];
            }
            merged.push(v);
        }
        merged.store(&args.dst)?;
    } else {
        let mut writer = BufWriter::new(std::fs::File::create(&args.dst)?);
        let mut perm = Vec::new();
        for path in args.perms {
            let p = JavaPermutation::mmap(&path, MmapFlags::RANDOM_ACCESS)?;
            perm.push(p);
        }
        let mut merged = Vec::new();

        ensure!(
            perm.iter()
                .all(|p| p.as_ref().len() == perm[0].as_ref().len()),
            "All permutations must have the same length"
        );

        for i in 0..perm[0].as_ref().len() {
            let mut v = i;
            for p in &perm {
                v = p.get(v);
            }
            merged.push(v);
        }
        for v in merged {
            writer.write_all(&(v as u64).to_be_bytes())?;
        }
    }
    log::info!("Completed in {} seconds", start.elapsed().as_secs_f64());
    Ok(())
}
