#[cfg(not(target_pointer_width = "64"))]
compile_error!("glint only supports 64-bit targets");

use std::env;
use std::fs;
use std::io::{self, Write};

#[derive(Clone, Copy)]
struct Rgb(u8, u8, u8);

impl Rgb {
    fn mix(self, other: Rgb, t: f32) -> Rgb {
        let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Rgb(
            channel(self.0, other.0),
            channel(self.1, other.1),
            channel(self.2, other.2),
        )
    }
}

const RED: Rgb = Rgb(0xFF, 0x55, 0x55);

struct Paint {
    enabled: bool,
}

impl Paint {
    fn wrap(&self, sgr: &str, text: &str) -> String {
        if self.enabled {
            format!("\x1b[{sgr}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn color(&self, c: Rgb, text: &str) -> String {
        self.wrap(&format!("38;2;{};{};{}", c.0, c.1, c.2), text)
    }

    fn bold(&self, c: Rgb, text: &str) -> String {
        self.wrap(&format!("1;38;2;{};{};{}", c.0, c.1, c.2), text)
    }

    fn dim(&self, text: &str) -> String {
        self.wrap("2", text)
    }
}

struct Theme {
    logo: &'static [&'static str],
    start: Rgb,
    end: Rgb,
}

const UBUNTU: &[&str] = &[
    r"           _  ",
    r"       ---(_) ",
    r"   _/  ---  \ ",
    r"  (_) |   |   ",
    r"    \  --- _/ ",
    r"       ---(_) ",
];

const ARCH: &[&str] = &[
    r"      /\      ",
    r"     /  \     ",
    r"    /\   \    ",
    r"   /      \   ",
    r"  /   ,,   \  ",
    r" /   |  |  -\ ",
    r"/_-''    ''-_\",
];

const DEBIAN: &[&str] = &[
    r"  _____   ",
    r" /  __ \  ",
    r"|  /    | ",
    r"|  \___-  ",
    r"-_        ",
    r"  --_     ",
];

const FEDORA: &[&str] = &[
    r"      _____   ",
    r"     /   __)\ ",
    r"     |  /  \ \",
    r"  ___|  |__/ /",
    r" / (_    _)_/ ",
    r"/ /  |  |     ",
    r"\ \__/  |     ",
    r" \(_____/     ",
];

const TUX: &[&str] = &[
    r"    ___     ",
    r"   (.. |    ",
    r"   (<> |    ",
    r"  / __  \   ",
    r" ( /  \ /|  ",
    r"_/\ __)/_)  ",
    r"\/-____\/   ",
];

fn theme(distro: &str) -> Theme {
    match distro {
        "ubuntu" => Theme {
            logo: UBUNTU,
            start: Rgb(0xE9, 0x54, 0x20),
            end: Rgb(0xA3, 0x3E, 0xC4),
        },
        "arch" | "endeavouros" | "manjaro" | "cachyos" => Theme {
            logo: ARCH,
            start: Rgb(0x17, 0x93, 0xD1),
            end: Rgb(0x7F, 0xE0, 0xFF),
        },
        "debian" => Theme {
            logo: DEBIAN,
            start: Rgb(0xD7, 0x0A, 0x53),
            end: Rgb(0xFF, 0x8F, 0xB1),
        },
        "fedora" => Theme {
            logo: FEDORA,
            start: Rgb(0x51, 0xA2, 0xDA),
            end: Rgb(0x29, 0x41, 0x72),
        },
        _ => Theme {
            logo: TUX,
            start: Rgb(0x00, 0xD9, 0xFF),
            end: Rgb(0xC0, 0x4C, 0xFF),
        },
    }
}

fn read(path: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn lookup<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    for line in text.lines() {
        let Some((k, v)) = line.split_once(['=', ':']) else {
            continue;
        };
        if k.trim() == key {
            return Some(v.trim().trim_matches('"'));
        }
    }
    None
}

fn count_dirs(path: &str, ignore: &[&str]) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| e.file_type().map_or(false, |t| t.is_dir()))
        .filter(|e| !ignore.iter().any(|name| e.file_name() == *name))
        .count()
}

fn human(kib: u64) -> String {
    let gib = kib as f64 / (1024.0 * 1024.0);
    if gib >= 1.0 {
        format!("{gib:.1}G")
    } else {
        format!("{}M", kib / 1024)
    }
}

fn username() -> String {
    if let Some(name) = env::var("USER").ok().or(env::var("LOGNAME").ok()) {
        return name;
    }
    let lookup_uid = || {
        let status = read("/proc/self/status")?;
        let uid = lookup(&status, "Uid")?.split_whitespace().next()?.to_string();
        let passwd = read("/etc/passwd")?;
        passwd.lines().find_map(|line| {
            let fields: Vec<&str> = line.split(':').collect();
            (fields.get(2) == Some(&uid.as_str())).then(|| fields[0].to_string())
        })
    };
    lookup_uid().unwrap_or_else(|| "user".into())
}

fn os() -> (String, String) {
    let release = read("/etc/os-release").unwrap_or_default();
    let id = lookup(&release, "ID").unwrap_or("linux").to_lowercase();
    let name = lookup(&release, "PRETTY_NAME")
        .or(lookup(&release, "NAME"))
        .unwrap_or("Linux");
    (id, format!("{name} {}", env::consts::ARCH))
}

fn is_placeholder(value: &str) -> bool {
    const PLACEHOLDERS: &[&str] = &[
        "system product name",
        "system version",
        "default string",
        "not specified",
        "not applicable",
        "type1productconfigid",
        "none",
    ];
    let value = value.to_lowercase();
    PLACEHOLDERS.contains(&value.as_str()) || value.contains("o.e.m")
}

fn host() -> Option<String> {
    let dmi = |field: &str| {
        read(&format!("/sys/devices/virtual/dmi/id/{field}")).filter(|s| !is_placeholder(s))
    };

    if let Some(product) = dmi("product_name") {
        return match dmi("product_version") {
            Some(version) if !product.contains(&version) => Some(format!("{product} {version}")),
            _ => Some(product),
        };
    }

    if let (Some(vendor), Some(board)) = (dmi("board_vendor"), dmi("board_name")) {
        let vendor = match vendor.as_str() {
            v if v.starts_with("ASUSTeK") => "ASUS",
            v if v.starts_with("Micro-Star") => "MSI",
            v if v.starts_with("Gigabyte") => "Gigabyte",
            v if v.starts_with("ASRock") => "ASRock",
            v => v,
        };
        return Some(format!("{vendor} {board}"));
    }

    let model = read("/sys/firmware/devicetree/base/model")?;
    Some(model.trim_end_matches('\0').to_string())
}

fn uptime() -> Option<String> {
    let raw = read("/proc/uptime")?;
    let secs: u64 = raw.split('.').next()?.parse().ok()?;
    let days = secs / 86400;
    let hours = secs / 3600 % 24;
    let mins = secs / 60 % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    parts.push(format!("{mins}m"));
    Some(parts.join(" "))
}

fn packages() -> Option<String> {
    let dpkg = fs::read("/var/lib/dpkg/status")
        .map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .lines()
                .filter(|l| l.starts_with("Status: ") && l.ends_with(" installed"))
                .count()
        })
        .unwrap_or(0);
    let counts = [
        (dpkg, "dpkg"),
        (count_dirs("/var/lib/pacman/local", &[]), "pacman"),
        (count_dirs("/snap", &["bin"]), "snap"),
        (count_dirs("/var/lib/flatpak/app", &[]), "flatpak"),
    ];

    let found: Vec<String> = counts
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, name)| format!("{n} {name}"))
        .collect();
    if found.is_empty() {
        None
    } else {
        Some(found.join(", "))
    }
}

fn shell() -> Option<String> {
    let path = env::var("SHELL").ok()?;
    Some(path.rsplit('/').next()?.to_string())
}

fn parent_pid(pid: &str) -> Option<String> {
    let stat = read(&format!("/proc/{pid}/stat"))?;
    // The process name (field 2) can contain spaces and ')', so split on the
    // last ')' rather than on whitespace.
    let after_name = stat.rsplit_once(')')?.1;
    Some(after_name.split_whitespace().nth(1)?.to_string())
}

fn process_name(pid: &str) -> Option<String> {
    let comm = read(&format!("/proc/{pid}/comm"))?;
    // The kernel cuts comm to 15 bytes ("gnome-terminal-server" becomes
    // "gnome-terminal-"), so recover the full name from argv[0] when that happens.
    if comm.len() == 15 {
        let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let argv0 = String::from_utf8_lossy(cmdline.split(|&b| b == 0).next().unwrap_or(&[]));
        let full = argv0.rsplit('/').next().unwrap_or("");
        if full.starts_with(&comm) {
            return Some(full.to_string());
        }
    }
    Some(comm)
}

fn terminal() -> Option<String> {
    match env::var("TERM_PROGRAM") {
        Ok(name) if !name.is_empty() && name != "tmux" => return Some(name),
        _ => {}
    }

    const SHELLS_AND_WRAPPERS: &[&str] = &[
        "bash", "zsh", "fish", "sh", "dash", "ksh", "mksh", "tcsh", "csh", "nu", "pwsh",
        "xonsh", "elvish", "sudo", "doas", "su", "login", "screen", "glint",
    ];
    let mut pid = String::from("self");
    for _ in 0..8 {
        let Some(parent) = parent_pid(&pid) else { break };
        pid = parent;
        let Some(name) = process_name(&pid) else { break };
        if pid == "0" || pid == "1" || name == "systemd" || name.starts_with("sshd") {
            break;
        }
        if !SHELLS_AND_WRAPPERS.contains(&name.as_str()) && !name.starts_with("tmux") {
            return Some(name);
        }
    }
    env::var("TERM").ok()
}

fn desktop(distro: &str) -> Option<String> {
    let current = env::var("XDG_CURRENT_DESKTOP").ok()?;
    let name = current
        .split(':')
        .find(|d| !d.eq_ignore_ascii_case(distro))
        .unwrap_or(&current);
    let name = name.strip_prefix("X-").unwrap_or(name);
    match env::var("XDG_SESSION_TYPE") {
        Ok(session) if !session.is_empty() => Some(format!("{name} ({session})")),
        _ => Some(name.to_string()),
    }
}

fn cpu() -> Option<String> {
    let info = fs::read_to_string("/proc/cpuinfo").ok()?;
    let threads = info.lines().filter(|l| l.starts_with("processor")).count();

    let raw = match lookup(&info, "model name") {
        Some(name) => name.to_string(),
        None => arm_core(&info)?,
    };

    let cleaned = raw.replace("(R)", "").replace("(TM)", "").replace(" CPU", "");
    let without_clock = cleaned.split(" @ ").next()?;
    let model: Vec<&str> = without_clock
        .split_whitespace()
        .filter(|w| !w.ends_with("-Core") && *w != "Processor")
        .collect();
    let model = model.join(" ");

    match read("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq") {
        Some(khz) => {
            let ghz = khz.parse::<f64>().ok()? / 1e6;
            Some(format!("{model} ({threads}) @ {ghz:.2}GHz"))
        }
        None => Some(format!("{model} ({threads})")),
    }
}

fn arm_core(cpuinfo: &str) -> Option<String> {
    if let Some(compatible) = read("/sys/firmware/devicetree/base/cpus/cpu@0/compatible") {
        let first = compatible.split('\0').next()?;
        let core = first.split_once(',').map_or(first, |(_, core)| core);
        let words: Vec<String> = core
            .split('-')
            .map(|w| {
                let mut chars = w.chars();
                chars.next().map_or(String::new(), |c| c.to_uppercase().chain(chars).collect())
            })
            .collect();
        return Some(words.join("-"));
    }

    if lookup(cpuinfo, "CPU implementer")? != "0x41" {
        return None;
    }
    let name = match lookup(cpuinfo, "CPU part")? {
        "0xd03" => "Cortex-A53",
        "0xd04" => "Cortex-A35",
        "0xd05" => "Cortex-A55",
        "0xd07" => "Cortex-A57",
        "0xd08" => "Cortex-A72",
        "0xd09" => "Cortex-A73",
        "0xd0a" => "Cortex-A75",
        "0xd0b" => "Cortex-A76",
        "0xd0c" => "Neoverse-N1",
        "0xd0d" => "Cortex-A77",
        "0xd40" => "Neoverse-V1",
        "0xd41" => "Cortex-A78",
        "0xd44" => "Cortex-X1",
        "0xd46" => "Cortex-A510",
        "0xd47" => "Cortex-A710",
        "0xd48" => "Cortex-X2",
        "0xd49" => "Neoverse-N2",
        "0xd4f" => "Neoverse-V2",
        _ => return None,
    };
    Some(name.to_string())
}

fn gpus() -> Vec<String> {
    let ids = fs::read_to_string("/usr/share/misc/pci.ids")
        .or_else(|_| fs::read_to_string("/usr/share/hwdata/pci.ids"))
        .unwrap_or_default();

    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let mut cards: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("card") && !name.contains('-'))
        .collect();
    cards.sort();

    let mut names = Vec::new();
    for card in cards {
        let id = |field: &str| {
            let raw = read(&format!("/sys/class/drm/{card}/device/{field}"))?;
            Some(raw.trim_start_matches("0x").to_lowercase())
        };
        let (Some(vendor), Some(device)) = (id("vendor"), id("device")) else {
            continue;
        };
        let name = pci_name(&ids, &vendor, &device);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn pci_name(ids: &str, vendor_id: &str, device_id: &str) -> String {
    let mut lines = ids.lines().skip_while(|l| !l.starts_with(vendor_id));
    let Some(vendor) = lines.next().and_then(|l| l.get(4..)) else {
        return format!("{vendor_id}:{device_id}");
    };
    let vendor = match vendor.trim() {
        v if v.contains("NVIDIA") => "NVIDIA",
        v if v.contains("AMD") || v.contains("ATI") => "AMD",
        v if v.starts_with("Intel") => "Intel",
        v => v,
    };

    let device = lines
        .take_while(|l| l.starts_with('\t') || l.starts_with('#'))
        .find(|l| l.strip_prefix('\t').map_or(false, |d| d.starts_with(device_id)))
        .and_then(|l| l.trim().get(4..));
    let Some(device) = device.map(str::trim) else {
        return format!("{vendor} {device_id}");
    };
    let device = match (device.find('['), device.rfind(']')) {
        (Some(open), Some(close)) if open < close => &device[open + 1..close],
        _ => device,
    };

    let device = device.strip_prefix("ATI ").unwrap_or(device);
    if device.starts_with(vendor) {
        device.to_string()
    } else {
        format!("{vendor} {device}")
    }
}

struct Usage {
    used: u64,
    total: u64,
    ratio: f64,
}

fn memory() -> Option<Usage> {
    let info = fs::read_to_string("/proc/meminfo").ok()?;
    let kib = |key| lookup(&info, key)?.trim_end_matches("kB").trim().parse::<u64>().ok();
    let total = kib("MemTotal")?;
    let available = match kib("MemAvailable") {
        Some(available) => available,
        None => kib("MemFree")? + kib("Buffers").unwrap_or(0) + kib("Cached").unwrap_or(0),
    };
    // Some containers fake /proc/meminfo and report more available than total.
    let used = total.saturating_sub(available);
    Some(Usage { used, total, ratio: used as f64 / total.max(1) as f64 })
}

// Mirrors glibc/musl `struct statvfs` on 64-bit targets only. On 32-bit the
// fields are narrower and this layout is wrong.
#[repr(C)]
struct StatVfs {
    bsize: u64,
    frsize: u64,
    blocks: u64,
    bfree: u64,
    bavail: u64,
    _rest: [u64; 9],
}

extern "C" {
    fn statvfs(path: *const u8, buf: *mut StatVfs) -> i32;
    fn isatty(fd: i32) -> i32;
}

fn disk(path: &str) -> Option<Usage> {
    let c_path = format!("{path}\0");
    let mut stat: StatVfs = unsafe { std::mem::zeroed() };
    if unsafe { statvfs(c_path.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    let kib = |blocks: u64| blocks * stat.frsize / 1024;
    let total = kib(stat.blocks);
    let used = total.saturating_sub(kib(stat.bfree));
    // Like df, measure against what non-root users can use, not the raw size,
    // since ext4 reserves about 5% for root.
    let usable = used + kib(stat.bavail);
    Some(Usage { used, total, ratio: used as f64 / usable.max(1) as f64 })
}

fn resolution() -> Option<String> {
    let mut modes = Vec::new();
    for entry in fs::read_dir("/sys/class/drm").ok()?.flatten() {
        let dir = entry.path().display().to_string();
        if read(&format!("{dir}/status")).as_deref() != Some("connected")
            || read(&format!("{dir}/enabled")).as_deref() == Some("disabled")
        {
            continue;
        }
        if let Some(list) = read(&format!("{dir}/modes")) {
            if let Some(preferred) = list.lines().next() {
                modes.push(preferred.to_string());
            }
        }
    }
    if modes.is_empty() {
        None
    } else {
        Some(modes.join(", "))
    }
}

fn battery() -> Option<String> {
    let mut dirs: Vec<String> = fs::read_dir("/sys/class/power_supply")
        .ok()?
        .flatten()
        .map(|e| e.path().display().to_string())
        .collect();
    dirs.sort();

    for dir in dirs {
        let field = |name: &str| read(&format!("{dir}/{name}"));
        // Wireless mice, keyboards and controllers are also type "Battery";
        // only scope "Device" tells them apart from the system battery.
        if field("type").as_deref() != Some("Battery") || field("scope").as_deref() == Some("Device") {
            continue;
        }
        let Some(capacity) = field("capacity") else { continue };
        let status = field("status").unwrap_or_default();
        return Some(format!("{capacity}% {}", status.to_lowercase()));
    }
    None
}

fn meter(paint: &Paint, ratio: f64, color: Rgb) -> String {
    const WIDTH: usize = 10;
    let ratio = ratio.clamp(0.0, 1.0);
    let filled = ((ratio * WIDTH as f64).round() as usize).min(WIDTH);
    let color = if ratio > 0.85 { RED } else { color };
    let empty = if paint.enabled { "━" } else { "─" };
    format!(
        "{}{} {}",
        paint.color(color, &"━".repeat(filled)),
        paint.dim(&empty.repeat(WIDTH - filled)),
        paint.dim(&format!("{:.0}%", ratio * 100.0)),
    )
}

struct Row {
    label: &'static str,
    value: String,
    ratio: Option<f64>,
}

impl Row {
    fn text(label: &'static str, value: Option<String>) -> Option<Row> {
        Some(Row { label, value: value?, ratio: None })
    }

    fn usage(label: &'static str, usage: Option<Usage>) -> Option<Row> {
        let usage = usage?;
        Some(Row {
            label,
            value: format!("{} / {}", human(usage.used), human(usage.total)),
            ratio: Some(usage.ratio),
        })
    }
}

fn collect_rows(distro: &str, os_name: String) -> Vec<Row> {
    let mut rows = vec![
        Row::text("os", Some(os_name)),
        Row::text("host", host()),
        Row::text("kernel", read("/proc/sys/kernel/osrelease")),
        Row::text("uptime", uptime()),
        Row::text("pkgs", packages()),
        Row::text("shell", shell()),
        Row::text("de", desktop(distro)),
        Row::text("term", terminal()),
        Row::text("res", resolution()),
        Row::text("cpu", cpu()),
    ];
    for gpu in gpus() {
        rows.push(Row::text("gpu", Some(gpu)));
    }
    rows.push(Row::usage("mem", memory()));
    rows.push(Row::usage("disk", disk("/")));
    rows.push(Row::text("bat", battery()));

    rows.into_iter().flatten().collect()
}

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let flag = |name: &str| args.iter().any(|a| a == name);

    if flag("-h") || flag("--help") {
        println!("glint: a tiny system fetch\n\nusage: glint [--no-color] [--no-logo]");
        return;
    }

    let no_color_env = env::var_os("NO_COLOR").map_or(false, |v| !v.is_empty());
    let is_terminal = unsafe { isatty(1) } == 1;
    let paint = Paint {
        enabled: is_terminal && !no_color_env && !flag("--no-color"),
    };
    let (distro, os_name) = os();
    let theme = theme(&distro);
    let logo: &[&str] = if flag("--no-logo") { &[] } else { theme.logo };

    let user = username();
    let hostname = read("/proc/sys/kernel/hostname").unwrap_or_else(|| "localhost".into());
    let rows = collect_rows(&distro, os_name);

    let mut info = vec![
        format!(
            "{}{}{}",
            paint.bold(theme.start, &user),
            paint.dim("@"),
            paint.bold(theme.end, &hostname),
        ),
        paint.dim(&"─".repeat(user.len() + 1 + hostname.len())),
    ];
    for (i, row) in rows.iter().enumerate() {
        let color = theme.start.mix(theme.end, i as f32 / rows.len() as f32);
        let mut line = format!("{} {}", paint.bold(color, &format!("{:>6}", row.label)), row.value);
        if let Some(ratio) = row.ratio {
            line.push_str("  ");
            line.push_str(&meter(&paint, ratio, color));
        }
        info.push(line);
    }
    if paint.enabled {
        let swatches: String = (0..8).map(|i| format!("\x1b[3{i}m● ")).collect();
        info.push(String::new());
        info.push(format!("       {swatches}\x1b[0m"));
    }

    let logo_width = logo.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let height = logo.len().max(info.len());
    let logo_top = (height - logo.len()) / 2;

    let mut out = String::from("\n");
    for i in 0..height {
        out.push_str("  ");
        if !logo.is_empty() {
            let row = i.checked_sub(logo_top).filter(|&j| j < logo.len());
            let line = row.map_or("", |j| logo[j]);
            let t = row.map_or(0.0, |j| j as f32 / (logo.len() - 1).max(1) as f32);
            let color = theme.start.mix(theme.end, t);
            out.push_str(&paint.bold(color, &format!("{line:<logo_width$}")));
            out.push_str("   ");
        }
        if let Some(line) = info.get(i) {
            out.push_str(line);
        }
        out.push('\n');
    }
    let _ = io::stdout().lock().write_all(out.as_bytes());
}
