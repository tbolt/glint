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
    r"               .-.",
    r"        .----.(   )",
    r"      .'      '-'`.",
    r"     /              \",
    r" .-.|                |",
    r"(   )                |",
    r" '-'|                |",
    r"     \              /",
    r"      '.      .-.,'",
    r"        '----(   )",
    r"              '-'",
];

const ARCH: &[&str] = &[
    r"           /\",
    r"          /  \",
    r"         /    \",
    r"        /\     \",
    r"       /  \     \",
    r"      /          \",
    r"     /            \",
    r"    /     .--.     \",
    r"   /     /    \     \",
    r"  /     |      |     \",
    r" /   _-'        '-_   \",
    r"/_-''              ''-_\",
];

const DEBIAN: &[&str] = &[
    r"     _.-----._",
    r"  .-'  _____  '-.",
    r" /   .'     '.   \",
    r"|   /    .-.  |   |",
    r"|  |    (   ) /   /",
    r"|   \    '-' /  .'",
    r" \   '.___.-' .'",
    r"  '.      _.-'",
    r"    '-.",
    r"       '-._",
    r"           '--.",
];

const FEDORA: &[&str] = &[
    r"       .------------.",
    r"    .-'              '-.",
    r"  .'         .----.     '.",
    r" /          /  .--'       \",
    r"|           | |            |",
    r"|      .----' '----.       |",
    r"|      '----. .----'       |",
    r"|           | |            |",
    r"|    .--.   | |            |",
    r" \   '.  '--' /          .'",
    r"  '.   '-.__.'         .'",
    r"    '-----------------'",
];

const TUX: &[&str] = &[
    r"    .--.",
    r"   |o_o |",
    r"   |:_/ |",
    r"  //   \ \",
    r" (|     | )",
    r"/'\_   _/`\",
    r"\___)=(___/",
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
        "debian" | "raspbian" => Theme {
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

fn os() -> (String, String) {
    let release = read("/etc/os-release").unwrap_or_default();
    let id = lookup(&release, "ID").unwrap_or("linux").to_lowercase();
    let name = lookup(&release, "PRETTY_NAME")
        .or(lookup(&release, "NAME"))
        .unwrap_or("Linux");
    (id, format!("{name} {}", env::consts::ARCH))
}

fn host() -> Option<String> {
    const PLACEHOLDERS: &[&str] = &[
        "System Product Name",
        "System Version",
        "To Be Filled By O.E.M.",
        "Default string",
        "None",
    ];
    let dmi = |field: &str| {
        read(&format!("/sys/devices/virtual/dmi/id/{field}"))
            .filter(|s| !PLACEHOLDERS.contains(&s.as_str()))
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
    let dpkg = fs::read_to_string("/var/lib/dpkg/status")
        .map(|s| s.matches("Status: install ok installed").count())
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

fn terminal() -> Option<String> {
    if let Ok(name) = env::var("TERM_PROGRAM") {
        return Some(name);
    }

    const NOT_TERMINALS: &[&str] = &[
        "bash", "zsh", "fish", "sh", "dash", "sudo", "su", "login", "screen", "glint",
    ];
    let mut pid = String::from("self");
    for _ in 0..8 {
        pid = parent_pid(&pid)?;
        if pid == "0" || pid == "1" {
            break;
        }
        let name = read(&format!("/proc/{pid}/comm"))?;
        if !NOT_TERMINALS.contains(&name.as_str()) && !name.starts_with("tmux") {
            return Some(name);
        }
    }
    env::var("TERM").ok()
}

fn desktop() -> Option<String> {
    let current = env::var("XDG_CURRENT_DESKTOP").ok()?;
    let name = current.rsplit(':').next().unwrap_or(&current);
    match env::var("XDG_SESSION_TYPE") {
        Ok(session) if !session.is_empty() => Some(format!("{name} ({session})")),
        _ => Some(name.to_string()),
    }
}

fn cpu() -> Option<String> {
    let info = fs::read_to_string("/proc/cpuinfo").ok()?;
    let raw = lookup(&info, "model name").or(lookup(&info, "Model"))?;
    let threads = info.lines().filter(|l| l.starts_with("processor")).count();

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
        let name = pci_name(&ids, &vendor, &device).unwrap_or(format!("{vendor}:{device}"));
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn pci_name(ids: &str, vendor_id: &str, device_id: &str) -> Option<String> {
    const AMD: &str = "1002";

    let mut lines = ids.lines().skip_while(|l| !l.starts_with(vendor_id));
    let vendor = lines.next()?.get(4..)?.trim();
    let vendor = match vendor {
        v if v.contains("NVIDIA") => "NVIDIA",
        v if v.contains("AMD") || v.contains("ATI") => "AMD",
        v if v.starts_with("Intel") => "Intel",
        v => v,
    };

    let device_line = lines
        .take_while(|l| l.starts_with('\t') || l.starts_with('#'))
        .find(|l| l.strip_prefix('\t').map_or(false, |d| d.starts_with(device_id)))?;
    let device = device_line.trim().get(4..)?.trim();
    let device = match (device.find('['), device.rfind(']')) {
        (Some(open), Some(close)) if open < close => &device[open + 1..close],
        _ => device,
    };

    if vendor_id == AMD && !device.contains("Radeon") {
        Some(format!("{vendor} {device} (iGPU)"))
    } else {
        Some(format!("{vendor} {device}"))
    }
}

fn memory() -> Option<(u64, u64)> {
    let info = fs::read_to_string("/proc/meminfo").ok()?;
    let kib = |key| lookup(&info, key)?.trim_end_matches("kB").trim().parse::<u64>().ok();
    let total = kib("MemTotal")?;
    let available = kib("MemAvailable")?;
    Some((total - available, total))
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
}

fn disk(path: &str) -> Option<(u64, u64)> {
    let c_path = format!("{path}\0");
    let mut stat: StatVfs = unsafe { std::mem::zeroed() };
    if unsafe { statvfs(c_path.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    let kib = |blocks: u64| blocks * stat.frsize / 1024;
    let total = kib(stat.blocks);
    Some((total - kib(stat.bfree), total))
}

fn resolution() -> Option<String> {
    let mut modes = Vec::new();
    for entry in fs::read_dir("/sys/class/drm").ok()?.flatten() {
        let dir = entry.path().display().to_string();
        if read(&format!("{dir}/status")).as_deref() != Some("connected") {
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
    for entry in fs::read_dir("/sys/class/power_supply").ok()?.flatten() {
        let dir = entry.path().display().to_string();
        if read(&format!("{dir}/type")).as_deref() != Some("Battery") {
            continue;
        }
        let capacity = read(&format!("{dir}/capacity"))?;
        let status = read(&format!("{dir}/status")).unwrap_or_default();
        return Some(format!("{capacity}% {}", status.to_lowercase()));
    }
    None
}

fn meter(paint: &Paint, used: u64, total: u64, color: Rgb) -> String {
    const WIDTH: usize = 10;
    let ratio = if total == 0 { 0.0 } else { used as f64 / total as f64 };
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
    usage: Option<(u64, u64)>,
}

impl Row {
    fn text(label: &'static str, value: Option<String>) -> Option<Row> {
        Some(Row { label, value: value?, usage: None })
    }

    fn usage(label: &'static str, usage: Option<(u64, u64)>) -> Option<Row> {
        let (used, total) = usage?;
        Some(Row {
            label,
            value: format!("{} / {}", human(used), human(total)),
            usage: Some((used, total)),
        })
    }
}

fn collect_rows(os_name: String) -> Vec<Row> {
    let mut rows = vec![
        Row::text("os", Some(os_name)),
        Row::text("host", host()),
        Row::text("kernel", read("/proc/sys/kernel/osrelease")),
        Row::text("uptime", uptime()),
        Row::text("pkgs", packages()),
        Row::text("shell", shell()),
        Row::text("de", desktop()),
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
    let args: Vec<String> = env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|a| a == name);

    if flag("-h") || flag("--help") {
        println!("glint: a tiny system fetch\n\nusage: glint [--no-color] [--no-logo]");
        return;
    }

    let paint = Paint {
        enabled: env::var_os("NO_COLOR").is_none() && !flag("--no-color"),
    };
    let (distro, os_name) = os();
    let theme = theme(&distro);
    let logo: &[&str] = if flag("--no-logo") { &[] } else { theme.logo };

    let user = env::var("USER").unwrap_or_else(|_| "user".into());
    let hostname = read("/proc/sys/kernel/hostname").unwrap_or_else(|| "localhost".into());
    let rows = collect_rows(os_name);

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
        if let Some((used, total)) = row.usage {
            line.push_str("  ");
            line.push_str(&meter(&paint, used, total, color));
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
            let line = i.checked_sub(logo_top).and_then(|j| logo.get(j)).unwrap_or(&"");
            let color = theme.start.mix(theme.end, i as f32 / height as f32);
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
