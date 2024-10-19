use anyhow::{bail, Context};
use bevy_color::Color;
use clap::ValueEnum;
use interprocess::local_socket::{
    prelude::*,
    traits::tokio::{Listener as _, Stream as _},
    GenericFilePath, ListenerOptions,
};
use std::{fs, io, process::ExitCode};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::cli::CliColorFormat;
use crate::formats::{self, CssColorFormat};

const SOCKET_NAME: &str = concat!(env!("CARGO_PKG_NAME"), ".sock");

pub fn start() -> ExitCode {
    let filename = if cfg!(target_os = "windows") {
        format!("\\\\.\\pipe\\{SOCKET_NAME}")
    } else {
        format!("/tmp/{SOCKET_NAME}")
    };

    if fs::exists(&filename).unwrap_or(false) {
        if let Err(err) = fs::remove_file(&filename) {
            eprintln!("Couldn't delete '{filename}': {err}");
            return ExitCode::FAILURE;
        }
    }

    let name = match filename.clone().to_fs_name::<GenericFilePath>() {
        Ok(name) => name,
        Err(e) => {
            eprintln!("Failed to create socket name: {e}");
            return ExitCode::FAILURE;
        }
    };

    let opts = ListenerOptions::new().name(name);

    println!("Server running at {filename}");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let listener = match opts.create_tokio() {
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                eprintln!(
                "Error: could not start server because the socket file is occupied. Please check if
    			{filename} is in use by another process and try again."
            );
                return ExitCode::FAILURE;
            }
            Err(e) => {
                eprintln!("Error: could not start server: {e}");
                return ExitCode::FAILURE;
            }
            Ok(l) => l,
        };

        loop {
            let conn = match listener.accept().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Incoming connection failed: {e}");
                    continue;
                }
            };

            tokio::spawn(async move {
                let (recv, mut send) = conn.split();
                let mut buffer = String::with_capacity(128);

                let mut recv = BufReader::new(recv);
                println!("Incoming connection!");

                loop {
                    match recv.read_line(&mut buffer).await {
                        Err(err) => {
                            eprintln!("Read failed: {err}");
                            break;
                        }
                        Ok(0) => {
                            println!("EOF reached, disconnecting client");
                            break;
                        }
                        _ => {}
                    }

                    println!("Got data {buffer}");

                    let Some(line) = buffer.strip_suffix("\n") else {
                        eprintln!("Read didn't end in newline!");
                        continue;
                    };

                    // let now = Instant::now();

                    match handle_message(line) {
                        Ok(response) => {
                            if let Err(err) = send.write_all(&response.into_bytes()).await {
                                eprintln!("Send failed: {err}");
                                return;
                            }
                        }
                        Err(err) => eprintln!("{err}"),
                    };
                    buffer.clear();

                    // println!("Sent in {:?}", now.elapsed());
                }
            });
        }
    })
}

fn handle_message(srt: &str) -> anyhow::Result<String> {
    if srt == "test" {
        bail!("test");
    }

    let (number, rest) = srt
        .split_once(":")
        .context("Read didn't contain the ':' delimiter !")?;

    let response_parts = rest.split("¿¿").map(|part| {
        let (fmt, color) = part
            .split_once(";")
            .context("Read didn't contain the ';' delimiter !")?;

        let number = number.parse::<u32>().context("invalid number")?;

        println!("Got {number}: color {color} with format {fmt}");

        let format_result =
            |color: Color| formats::format_color(color.into(), CssColorFormat::Hex.into(), true);

        let response = if fmt == "auto" {
            match formats::parse_color_unknown_format(color) {
                Some((color, _, _)) => format_result(color),
                None => "ERR".into(),
            }
        } else if let Ok(fmt) = CliColorFormat::from_str(fmt, true) {
            match formats::parse_color(color, fmt.into()) {
                Some((color, _)) => format_result(color),
                None => "ERR".into(),
            }
        } else {
            "ERR".into()
        };

        Ok(response)
    });
    let response = format!(
        "{}:{}\n",
        number,
        response_parts
            .collect::<anyhow::Result<Vec<_>>>()?
            .join("¿¿")
    );

    println!("Sending response {response}");
    Ok(response)
}
