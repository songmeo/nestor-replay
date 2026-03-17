use anyhow::{Context, Result};
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use socketcan::{CanSocket, EmbeddedFrame, ExtendedId, Frame, Socket, StandardId};
use std::time::{Duration, Instant};

struct Output {
    enabled: bool,
}

impl Output {
    fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    fn println(&self, msg: impl AsRef<str>) {
        if self.enabled {
            println!("{}", msg.as_ref());
        }
    }

    fn printf(&self, args: std::fmt::Arguments<'_>) {
        if self.enabled {
            println!("{}", args);
        }
    }

    fn progress_bar(&self, len: u64) -> Option<ProgressBar> {
        if !self.enabled {
            return None;
        }

        let pb = ProgressBar::new(len);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} frames ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    }
}

/// Replay CAN recordings from CyphalCloud/Nestor server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Nestor server URL
    #[arg(short, long, default_value = "https://cyphalcloud.zubax.com")]
    server: String,

    /// Device name (skip interactive selection)
    #[arg(short, long)]
    device: Option<String>,

    /// Boot ID (skip interactive selection)
    #[arg(short, long)]
    boot: Option<u64>,

    /// SocketCAN interface
    #[arg(short, long, default_value = "vcan0")]
    interface: String,

    /// Playback speed multiplier
    #[arg(long, default_value = "1.0")]
    speed: f64,

    /// Don't send to CAN, just display
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct DevicesResponse {
    devices: Vec<DeviceDTO>,
}

#[derive(Debug, Deserialize)]
struct DeviceDTO {
    device: String,
    last_heard_ts: i64,
    last_uid: u64,
}

#[derive(Debug, Deserialize)]
struct BootsResponse {
    device: String,
    boots: Vec<BootDTO>,
}

#[derive(Debug, Deserialize)]
struct BootDTO {
    boot_id: u64,
    first_record: CANFrameRecordDTO,
    last_record: CANFrameRecordDTO,
}

#[derive(Debug, Deserialize)]
struct RecordsResponse {
    device: String,
    latest_seqno_seen: Option<i64>,
    records: Vec<CANFrameRecordDTO>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CANFrameRecordDTO {
    pub hw_ts_us: i64,
    pub boot_id: u64,
    pub seqno: i64,
    pub commit_ts: i64,
    pub frame: CANFrameDTO,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CANFrameDTO {
    pub can_id: u32,
    pub extended: bool,
    pub rtr: bool,
    pub error: bool,
    pub data_hex: String,
}

fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let out = Output::new(args.dry_run);
    let client = reqwest::blocking::Client::new();
    let theme = ColorfulTheme::default();

    out.printf(format_args!("Connecting to {}...\n", args.server));

    // Get devices
    let devices: DevicesResponse = client
        .get(format!("{}/cf3d/api/v1/devices", args.server))
        .send()
        .context("Failed to connect to server")?
        .json()
        .context("Failed to parse devices response")?;

    if devices.devices.is_empty() {
        out.println("No devices found on server.");
        return Ok(());
    }

    // Select device
    let device_name = if let Some(ref name) = args.device {
        name.clone()
    } else {
        let items: Vec<String> = devices
            .devices
            .iter()
            .map(|d| {
                format!(
                    "{} (last seen: {})",
                    d.device,
                    format_timestamp(d.last_heard_ts)
                )
            })
            .collect();

        let selection = Select::with_theme(&theme)
            .with_prompt("Select device")
            .items(&items)
            .default(0)
            .interact()?;

        devices.devices[selection].device.clone()
    };

    out.printf(format_args!("Selected device: {}\n", device_name));

    // Get boots
    let boots: BootsResponse = client
        .get(format!(
            "{}/cf3d/api/v1/boots?device={}",
            args.server,
            urlencoding::encode(&device_name)
        ))
        .send()
        .context("Failed to fetch boots")?
        .json()
        .context("Failed to parse boots response")?;

    if boots.boots.is_empty() {
        out.printf(format_args!(
            "No boot sessions found for device '{}'.",
            device_name
        ));
        return Ok(());
    }

    // Select boot
    let boot_id = if let Some(id) = args.boot {
        id
    } else {
        let items: Vec<String> = boots
            .boots
            .iter()
            .map(|b| {
                let first_ts = format_timestamp(b.first_record.commit_ts);
                let last_ts = format_timestamp(b.last_record.commit_ts);
                let frames = b.last_record.seqno - b.first_record.seqno + 1;
                format!(
                    "Boot #{} ({} to {}, {} frames)",
                    b.boot_id, first_ts, last_ts, frames
                )
            })
            .collect();

        let selection = Select::with_theme(&theme)
            .with_prompt("Select boot session")
            .items(&items)
            .default(0)
            .interact()?;

        boots.boots[selection].boot_id
    };

    out.printf(format_args!("Selected boot: #{}\n", boot_id));

    // Fetch all records (paginated)
    out.println("Fetching records...");
    let mut all_records: Vec<CANFrameRecordDTO> = Vec::new();
    let mut seqno_min: Option<i64> = None;

    loop {
        let mut url = format!(
            "{}/cf3d/api/v1/records?device={}&boot_id={}&limit=10000",
            args.server,
            urlencoding::encode(&device_name),
            boot_id
        );
        if let Some(min) = seqno_min {
            url.push_str(&format!("&seqno_min={}", min));
        }

        let response: RecordsResponse = client
            .get(&url)
            .send()
            .context("Failed to fetch records")?
            .json()
            .context("Failed to parse records response")?;

        let count = response.records.len();
        if count == 0 {
            break;
        }

        let last_seqno = response.records.last().map(|r| r.seqno).unwrap_or(0);
        all_records.extend(response.records);
        out.printf(format_args!(
            "  Fetched {} records (total: {})",
            count,
            all_records.len()
        ));

        if count < 10000 {
            break;
        }
        seqno_min = Some(last_seqno + 1);
    }

    if all_records.is_empty() {
        out.printf(format_args!("No records found for boot #{}.", boot_id));
        return Ok(());
    }

    // Sort by hardware timestamp
    all_records.sort_by_key(|r| r.hw_ts_us);

    out.printf(format_args!(
        "\nReplaying {} frames to {} at {:.1}x speed{}...\n",
        all_records.len(),
        args.interface,
        args.speed,
        if args.dry_run { " (dry run)" } else { "" }
    ));

    // Open SocketCAN interface
    let socket = if !args.dry_run {
        Some(CanSocket::open(&args.interface).context(format!(
            "Failed to open SocketCAN interface '{}'. Make sure it exists (ip link show {})",
            args.interface, args.interface
        ))?)
    } else {
        None
    };

    // Progress bar (only in dry-run; non-dry-run should avoid stdout work)
    let pb = out.progress_bar(all_records.len() as u64);

    let start_time = Instant::now();
    let mut prev_hw_ts: Option<i64> = None;
    let mut frames_sent = 0u64;

    for record in &all_records {
        // Calculate delay
        if let Some(prev_ts) = prev_hw_ts {
            let delay_us = ((record.hw_ts_us - prev_ts) as f64 / args.speed) as u64;
            if delay_us > 0 && delay_us < 10_000_000 {
                // Cap at 10 seconds
                std::thread::sleep(Duration::from_micros(delay_us));
            }
        }
        prev_hw_ts = Some(record.hw_ts_us);

        // Decode frame data
        let data = hex::decode(&record.frame.data_hex).unwrap_or_default();

        // Create and send frame
        if let Some(ref sock) = socket {
            let frame = if record.frame.extended {
                let id = ExtendedId::new(record.frame.can_id).context("Invalid extended CAN ID")?;
                socketcan::CanDataFrame::new(id, &data).context("Failed to create CAN frame")?
            } else {
                let id = StandardId::new(record.frame.can_id as u16)
                    .context("Invalid standard CAN ID")?;
                socketcan::CanDataFrame::new(id, &data).context("Failed to create CAN frame")?
            };
            sock.write_frame(&frame).context("Failed to send frame")?;
        }

        if out.enabled {
            // Print candump-style output
            let elapsed = start_time.elapsed().as_secs_f64();
            let id_str = if record.frame.extended {
                format!("{:08X}", record.frame.can_id)
            } else {
                format!("{:03X}", record.frame.can_id)
            };
            let data_str: Vec<String> = data.iter().map(|b| format!("{:02X}", b)).collect();

            println!(
                "[{:8.3}s] {}  {} [{}]  {}",
                elapsed,
                args.interface,
                id_str,
                data.len(),
                data_str.join(" ")
            );
        }

        frames_sent += 1;
        if let Some(ref pb) = pb {
            pb.set_position(frames_sent);
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message("Done!");
    }

    let total_time = start_time.elapsed();

    // In dry-run, stdout already contains frame-by-frame output, so also print a summary.
    // In non-dry-run, remain silent during replay, but print a single final summary line.
    println!(
        "Replayed {} frames in {:.1}s{}",
        frames_sent,
        total_time.as_secs_f64(),
        if args.dry_run { " (dry run)" } else { "" }
    );

    Ok(())
}
