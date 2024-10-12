use std::{
    fmt::Write,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result as AResult};
use bytesize::ByteSize;
use clap::{Parser, ValueEnum};
use ffmpeg::format;
use ffmpeg_the_third as ffmpeg;
use indicatif::{ParallelProgressIterator, ProgressState, ProgressStyle};
use owo_colors::{
    colors::{BrightGreen, BrightRed},
    OwoColorize,
};
use rayon::prelude::*;
use walkdir::WalkDir;

mod transcode;
use transcode::transcoder;

#[derive(Debug, Clone, ValueEnum, Copy)]
pub enum OutputCodec {
    Mp3,
    Opus,
    Aac,
}

/// Transcodes your music folder without asking too many questions
#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
struct Args {
    src: PathBuf,
    out: PathBuf,

    #[clap(short = 'c', long, default_value_t = OutputCodec::Mp3, value_enum)]
    codec: OutputCodec,

    #[clap(short = 'i', long)]
    show_errors: bool,
}

fn main() -> AResult<()> {
    let args = Args::parse();
    ffmpeg::init()?;
    ffmpeg::log::set_level(ffmpeg::log::Level::Fatal);

    fs::create_dir(&args.out).ok();
    let files = WalkDir::new(&args.src).into_iter().collect::<Vec<_>>();

    let ignored_exts = vec![
        "m3u", "log", "torrent", "cue", "part", "db", "pdf", "reapeaks",
    ];

    let input_size = AtomicU64::new(0);
    let output_size = AtomicU64::new(0);

    let errors = files
        .into_par_iter()
        .progress_with_style(
            ProgressStyle::with_template(
                "({human_pos}/{human_len} :: {eta}) [{wide_bar:.cyan/blue}] [{elapsed_precise}]",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
            .progress_chars("#>-"),
        )
        .filter_map(|rf| rf.ok())
        .filter(|f| {
            f.file_type().is_file()
                && !ignored_exts.contains(
                    &f.path()
                        .extension()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default(),
                )
        })
        .map(|f| {
            let input_path = f.path().to_path_buf();
            if let Ok(metadata) = input_path.metadata() {
                input_size.fetch_add(metadata.len(), Ordering::Release);
            }

            let base_comps = args.src.canonicalize().unwrap().components().count();

            let mut rel_comps = PathBuf::new();
            input_path
                .canonicalize()
                .unwrap()
                .components()
                .enumerate()
                .filter(|(i, _)| *i >= base_comps)
                .for_each(|(_, c)| rel_comps.push(c));

            let output_path_orig_ext = args.out.join(rel_comps);
            let output_path = output_path_orig_ext.with_extension(match args.codec {
                OutputCodec::Mp3 => "mp3",
                OutputCodec::Opus => "opus",
                OutputCodec::Aac => "m4a",
            });
            (
                (input_path.clone(), output_path.clone()),
                (|| -> AResult<()> {
                    // create folders if missing
                    fs::create_dir_all(output_path.parent().context("get output_path parent")?)
                        .ok();

                    // skip if present
                    if output_path.exists() {
                        // still calculate size..
                        if let Ok(metadata) = output_path.metadata() {
                            output_size.fetch_add(metadata.len(), Ordering::Release);
                        }
                        return Ok(());
                    }

                    let mut ictx = format::input(&input_path)?;
                    let mut octx = format::output(&output_path)?;

                    let mut transcoder = transcoder(&mut ictx, &mut octx, &output_path, "anull", args.codec)?;
                    octx.set_metadata(ictx.metadata().to_owned());
                    octx.write_header()?;

                    for (stream, mut packet) in ictx.packets().filter_map(Result::ok) {
                        if stream.index() == transcoder.stream {
                            packet.rescale_ts(stream.time_base(), transcoder.in_time_base);
                            if let Err(e) = transcoder.send_packet_to_decoder(&packet) {
                                // if we failed to decode this file,
                                // we'll just abort this transcode and copy it raw.
                                fs::remove_file(&output_path).ok();
                                let copied = fs::copy(&input_path, &output_path_orig_ext)?;
                                output_size.fetch_add(copied, Ordering::Release);
                                // danke schön
                                return Err(e.context("skipped file and copied it instead"));
                            }
                            transcoder.receive_and_process_decoded_frames(&mut octx)?;
                        }
                    }

                    transcoder.send_eof_to_decoder()?;
                    transcoder.receive_and_process_decoded_frames(&mut octx)?;

                    transcoder.flush_filter()?;
                    transcoder.get_and_process_filtered_frames(&mut octx)?;

                    transcoder.send_eof_to_encoder()?;
                    transcoder.receive_and_process_encoded_packets(&mut octx)?;

                    octx.write_trailer()?;
                    unsafe {
                        octx.destructor();
                    }
                    drop(octx);

                    if let Ok(metadata) = output_path.metadata() {
                        output_size.fetch_add(metadata.len(), Ordering::Release);
                    }
                    Ok(())
                })(),
            )
        })
        .filter(|(_, r)| r.is_err())
        .map(|(io, r)| (io, r.err().unwrap()))
        .collect::<Vec<_>>();

    {
        let hint = if args.show_errors {
            format!("")
        } else {
            format!(" {}", "Run with --show-errors [-i] for details".italic())
        };
        eprintln!("{} failed:{hint}", errors.len());
    }
    errors.iter().for_each(|((input_path, output_path), e)| {
        if args.show_errors {
            eprintln!("\t'{}':\n\t\t{}", input_path.to_string_lossy(), e.red());
        }

        fs::remove_file(output_path).ok();
    });

    eprintln!("");

    {
        let i = input_size.load(Ordering::SeqCst);
        let o = output_size.load(Ordering::SeqCst);

        let percent = (1. - ((o as f64) / (i as f64))) * 100. * -1.;

        eprintln!(
            "Transcoded {} ({}%)",
            format!("{} ⇒ {}", ByteSize(i).green(), ByteSize(o).green()).bold(),
            if percent < 0. {
                format!("{:.2}", percent.fg::<BrightGreen>())
            } else {
                format!("{:.2}", percent.fg::<BrightRed>())
            }
        );
    }

    Ok(())
}
