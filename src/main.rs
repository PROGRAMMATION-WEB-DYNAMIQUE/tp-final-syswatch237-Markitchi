// src/main.rs
use chrono::Local;
use std::fmt;
use sysinfo::{System, Process};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::fs::OpenOptions;

// Forcer la console Windows en UTF-8 (codepage 65001)
#[cfg(windows)]
fn setup_utf8_console() {
    std::process::Command::new("cmd")
        .args(["/C", "chcp", "65001"])
        .output()
        .ok();
}

#[cfg(not(windows))]
fn setup_utf8_console() {}

const AUTH_TOKEN: &str = "ENSPD2026";

// --- Types metier ---

#[derive(Debug, Clone)]
struct CpuInfo {
    usage_percent: f32,
    core_count: usize,
}

#[derive(Debug, Clone)]
struct MemInfo {
    total_mb: u64,
    used_mb: u64,
    free_mb: u64,
}

#[derive(Debug, Clone)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory_mb: u64,
}

#[derive(Debug, Clone)]
struct SystemSnapshot {
    timestamp: String,
    cpu: CpuInfo,
    memory: MemInfo,
    top_processes: Vec<ProcessInfo>,
}

// --- Affichage humain (Trait Display) ---

impl fmt::Display for CpuInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CPU: {:.1}% ({} coeurs)", self.usage_percent, self.core_count)
    }
}

impl fmt::Display for MemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MEM: {}MB utilises / {}MB total ({} MB libres)",
            self.used_mb, self.total_mb, self.free_mb
        )
    }
}

impl fmt::Display for ProcessInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "  [{:>6}] {:<25} CPU:{:>5.1}%  MEM:{:>5}MB",
            self.pid, self.name, self.cpu_usage, self.memory_mb
        )
    }
}

impl fmt::Display for SystemSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== SysWatch -- {} ===", self.timestamp)?;
        writeln!(f, "{}", self.cpu)?;
        writeln!(f, "{}", self.memory)?;
        writeln!(f, "--- Top Processus ---")?;
        for p in &self.top_processes {
            writeln!(f, "{}", p)?;
        }
        write!(f, "=====================")
    }
}

// --- Erreurs custom --- Etape 2: Gestion d'erreurs avec un enum dedie

#[derive(Debug)]
enum SysWatchError {
    CollectionFailed(String),
}

impl fmt::Display for SysWatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SysWatchError::CollectionFailed(msg) => write!(f, "Erreur collecte: {}", msg),
        }
    }
}

impl std::error::Error for SysWatchError {}

// --- Collecte systeme ---

fn collect_snapshot() -> Result<SystemSnapshot, SysWatchError> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Petite pause pour que sysinfo ait des valeurs CPU non nulles
    std::thread::sleep(std::time::Duration::from_millis(500));
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_info().cpu_usage();
    let core_count = sys.cpus().len();

    if core_count == 0 {
        return Err(SysWatchError::CollectionFailed("Aucun CPU detecte".to_string()));
    }

    let total_mb = sys.total_memory() / 1024 / 1024;
    let used_mb = sys.used_memory() / 1024 / 1024;
    let free_mb = sys.free_memory() / 1024 / 1024;

    // Top 5 processus par consommation CPU
    let mut processes: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p: &Process| ProcessInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string(),
            cpu_usage: p.cpu_usage(),
            memory_mb: p.memory() / 1024 / 1024,
        })
        .collect();

    processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
    processes.truncate(5);

    Ok(SystemSnapshot {
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        cpu: CpuInfo { usage_percent: cpu_usage, core_count },
        memory: MemInfo { total_mb, used_mb, free_mb },
        top_processes: processes,
    })
}

// Formatage des reponses reseau

fn format_response(snapshot: &SystemSnapshot, command: &str) -> String {
    let cmd = command.trim().to_lowercase();

    match cmd.as_str() {
        "cpu" => format!(
            "[CPU]\n{}\n\nHistorique:\n{}\n",
            snapshot.cpu,
            (0..10)
                .map(|i| {
                    let threshold = (snapshot.cpu.usage_percent / 10.0) as usize;
                    if i < threshold { "#" } else { "." }
                })
                .collect::<Vec<_>>()
                .join("") + &format!(" {:.1}%", snapshot.cpu.usage_percent)
        ),

        "mem" => {
            let percent = (snapshot.memory.used_mb as f64 / snapshot.memory.total_mb as f64) * 100.0;
            let bar: String = (0..20)
                .map(|i| if i < (percent / 5.0) as usize { '#' } else { '.' })
                .collect();
            format!(
                "[MEMOIRE]\n{}\n[{}] {:.1}%\n",
                snapshot.memory, bar, percent
            )
        },

        "ps" | "procs" => {
            let lines: String = snapshot
                .top_processes
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{}. {}", i + 1, p))
                .collect::<Vec<_>>()
                .join("\n");
            format!("[PROCESSUS -- Top {}]\n{}\n", snapshot.top_processes.len(), lines)
        },

        "shutdown" => {
            std::process::Command::new("shutdown")
                .args(["/s", "/t", "5"])
                .spawn()
                .ok();
            "SHUTDOWN programme dans 5 secondes.\n".to_string()
        }

        "reboot" => {
            std::process::Command::new("shutdown")
                .args(["/r", "/t", "5"])
                .spawn()
                .ok();
            "REBOOT programme dans 5 secondes.\n".to_string()
        }

        "abort" => {
            std::process::Command::new("shutdown")
                .args(["/a"])
                .spawn()
                .ok();
            "Extinction annulee.\n".to_string()
        }

        _ if cmd.starts_with("msg ") => {
            let text = &cmd[4..];
            println!("\n+======================================+");
            println!("| MESSAGE DU PROFESSEUR                |");
            println!("| {}{}|", text, " ".repeat(38usize.saturating_sub(text.len())));
            println!("+======================================+\n");
            format!("Message affiche sur la machine cible.\n")
        }

        _ if cmd.starts_with("install ") => {
            let package = cmd[8..].trim().to_string();
            std::thread::spawn(move || {
                std::process::Command::new("winget")
                    .args(["install", "--silent", &package])
                    .status()
                    .ok();
            });
            format!("Installation de '{}' lancee en arriere-plan.\n", &cmd[8..])
        }

        _ if cmd.starts_with("exec ") => {
            let shell_cmd = &cmd[5..];
            match std::process::Command::new("cmd")
                .args(["/C", shell_cmd])
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut result = String::new();
                    if !stdout.is_empty() {
                        result.push_str(&stdout);
                    }
                    if !stderr.is_empty() {
                        result.push_str("[STDERR] ");
                        result.push_str(&stderr);
                    }
                    if result.is_empty() {
                        result = "(commande executee, pas de sortie)\n".to_string();
                    }
                    result
                }
                Err(e) => format!("Erreur execution: {}\n", e),
            }
        }

        _ if cmd.starts_with("kill ") => {
            let pid_str = cmd[5..].trim();
            match pid_str.parse::<u32>() {
                Ok(pid) => {
                    match std::process::Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/F"])
                        .output()
                    {
                        Ok(output) => {
                            let out = String::from_utf8_lossy(&output.stdout);
                            let err = String::from_utf8_lossy(&output.stderr);
                            if output.status.success() {
                                format!("Processus {} termine.\n{}", pid, out)
                            } else {
                                format!("Echec kill PID {}: {}\n", pid, err)
                            }
                        }
                        Err(e) => format!("Erreur taskkill: {}\n", e),
                    }
                }
                Err(_) => format!("PID invalide: '{}'\n", pid_str),
            }
        }

        "lock" => {
            std::process::Command::new("rundll32.exe")
                .args(["user32.dll,LockWorkStation"])
                .spawn()
                .ok();
            "Poste verrouille.\n".to_string()
        }

        "users" => {
            match std::process::Command::new("query").arg("user").output() {
                Ok(output) => {
                    let out = String::from_utf8_lossy(&output.stdout);
                    if out.is_empty() {
                        "Aucun utilisateur connecte.\n".to_string()
                    } else {
                        format!("[UTILISATEURS]\n{}\n", out)
                    }
                }
                Err(e) => format!("Erreur query user: {}\n", e),
            }
        }

        "all" | "" => format!("{}\n", snapshot),

        "help" => [
            "Commandes disponibles:",
            "  cpu      - Usage CPU + barre",
            "  mem      - Memoire RAM",
            "  ps       - Top processus",
            "  all      - Vue complete",
            "  exec <c> - Executer une commande shell",
            "  kill <p> - Tuer un processus par PID",
            "  lock     - Verrouiller le poste",
            "  users    - Lister les utilisateurs",
            "  msg <t>  - Afficher un message",
            "  install  - Installer un logiciel",
            "  shutdown - Eteindre la machine",
            "  reboot   - Redemarrer la machine",
            "  abort    - Annuler extinction",
            "  help     - Cette aide",
            "  quit     - Fermer la connexion\n",
        ].join("\n"),

        "quit" | "exit" => "BYE\n".to_string(),

        _ => format!("Commande inconnue: '{}'. Tape 'help'.\n", command.trim()),
    }
}


fn snapshot_refresher(snapshot: Arc<Mutex<SystemSnapshot>>) {
    loop {
        thread::sleep(Duration::from_secs(5));
        match collect_snapshot() {
            Ok(new_snap) => {
                let mut snap = snapshot.lock().unwrap();
                *snap = new_snap;
                println!("[refresh] Metriques mises a jour");
            }
            Err(e) => eprintln!("[refresh] Erreur: {}", e),
        }
    }
}


fn log_event(message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("[{}] {}\n", timestamp, message);

    print!("{}", line);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("syswatch.log")
    {
        let _ = file.write_all(line.as_bytes());
    }
}


fn handle_client(mut stream: TcpStream, snapshot: Arc<Mutex<SystemSnapshot>>) {
    let peer = stream.peer_addr()
        .map(|a| a.to_string())
        .unwrap_or("inconnu".to_string());
    log_event(&format!("[+] Connexion de {}", peer));

    let _ = stream.write_all(b"TOKEN: ");
    let mut reader = BufReader::new(stream.try_clone().expect("Clone failed"));
    let mut token_line = String::new();
    if reader.read_line(&mut token_line).is_err() || token_line.trim() != AUTH_TOKEN {
        let _ = stream.write_all(b"UNAUTHORIZED\n");
        log_event(&format!("[!] Acces refuse depuis {}", peer));
        return;
    }
    let _ = stream.write_all(b"OK\n");
    log_event(&format!("[OK] Authentifie: {}", peer));

    for line in reader.lines() {
        match line {
            Ok(cmd) => {
                let cmd = cmd.trim().to_string();
                log_event(&format!("[{}] commande: '{}'", peer, cmd));

                if cmd.eq_ignore_ascii_case("quit") {
                    let _ = stream.write_all(b"BYE\n");
                    break;
                }

                let response = {
                    let snap = snapshot.lock().unwrap();
                    format_response(&snap, &cmd)
                };

                let _ = stream.write_all(response.as_bytes());
                let _ = stream.write_all(b"\nEND\n");
            }
            Err(_) => break,
        }
    }

    log_event(&format!("[-] Deconnexion de {}", peer));
}


fn main() {
    setup_utf8_console();
    println!("SysWatch demarrage...");

    let initial = collect_snapshot().expect("Impossible de collecter les metriques initiales");
    println!("Metriques initiales OK:\n{}", initial);

    let shared_snapshot = Arc::new(Mutex::new(initial));

    {
        let snap_clone = Arc::clone(&shared_snapshot);
        thread::spawn(move || snapshot_refresher(snap_clone));
    }

    let listener = TcpListener::bind("0.0.0.0:7878").expect("Impossible de bind le port 7878");
    println!("Serveur en ecoute sur port 7878...");
    println!("Connecte-toi avec: telnet localhost 7878");
    println!("  ou: nc localhost 7878 (WSL/Git Bash)");
    println!("Ctrl+C pour arreter.\n");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let snap_clone = Arc::clone(&shared_snapshot);
                thread::spawn(move || handle_client(stream, snap_clone));
            }
            Err(e) => eprintln!("Erreur connexion entrante: {}", e),
        }
    }
}


// ===================== TESTS =====================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_snapshot() -> SystemSnapshot {
        SystemSnapshot {
            timestamp: "2026-01-01 00:00:00".to_string(),
            cpu: CpuInfo { usage_percent: 45.0, core_count: 4 },
            memory: MemInfo { total_mb: 16384, used_mb: 8192, free_mb: 8192 },
            top_processes: vec![
                ProcessInfo { pid: 1000, name: "chrome.exe".to_string(), cpu_usage: 12.5, memory_mb: 512 },
                ProcessInfo { pid: 2000, name: "code.exe".to_string(), cpu_usage: 8.3, memory_mb: 256 },
            ],
        }
    }

    #[test]
    fn test_cpu_info_display() {
        let cpu = CpuInfo { usage_percent: 42.5, core_count: 8 };
        let display = format!("{}", cpu);
        assert!(display.contains("42.5%"));
        assert!(display.contains("8"));
    }

    #[test]
    fn test_mem_info_display() {
        let mem = MemInfo { total_mb: 16384, used_mb: 8192, free_mb: 8192 };
        let display = format!("{}", mem);
        assert!(display.contains("8192"));
        assert!(display.contains("16384"));
    }

    #[test]
    fn test_process_info_display() {
        let proc_info = ProcessInfo { pid: 1234, name: "test.exe".to_string(), cpu_usage: 5.5, memory_mb: 128 };
        let display = format!("{}", proc_info);
        assert!(display.contains("1234"));
        assert!(display.contains("test.exe"));
        assert!(display.contains("5.5"));
    }

    #[test]
    fn test_snapshot_display() {
        let snap = make_test_snapshot();
        let display = format!("{}", snap);
        assert!(display.contains("SysWatch"));
        assert!(display.contains("2026-01-01"));
        assert!(display.contains("chrome.exe"));
    }

    #[test]
    fn test_format_cpu() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "cpu");
        assert!(result.contains("[CPU]"));
        assert!(result.contains("45.0%"));
    }

    #[test]
    fn test_format_mem() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "mem");
        assert!(result.contains("[MEMOIRE]") || result.contains("MEM"));
    }

    #[test]
    fn test_format_ps() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "ps");
        assert!(result.contains("PROCESSUS"));
        assert!(result.contains("chrome.exe"));
    }

    #[test]
    fn test_format_all() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "all");
        assert!(result.contains("SysWatch"));
    }

    #[test]
    fn test_format_help() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "help");
        assert!(result.contains("cpu"));
        assert!(result.contains("mem"));
        assert!(result.contains("exec"));
        assert!(result.contains("kill"));
        assert!(result.contains("lock"));
        assert!(result.contains("quit"));
    }

    #[test]
    fn test_format_quit() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "quit");
        assert_eq!(result, "BYE\n");
    }

    #[test]
    fn test_format_unknown_command() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "xyz123");
        assert!(result.contains("Commande inconnue"));
        assert!(result.contains("xyz123"));
    }

    #[test]
    fn test_format_command_case_insensitive() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "CPU");
        assert!(result.contains("[CPU]"));
    }

    #[test]
    fn test_collect_snapshot_succeeds() {
        let result = collect_snapshot();
        assert!(result.is_ok(), "collect_snapshot devrait reussir");
        let snap = result.unwrap();
        assert!(snap.cpu.core_count > 0);
        assert!(snap.memory.total_mb > 0);
        assert!(snap.top_processes.len() <= 5);
    }

    #[test]
    fn test_snapshot_processes_sorted_by_cpu() {
        let result = collect_snapshot().unwrap();
        for w in result.top_processes.windows(2) {
            assert!(w[0].cpu_usage >= w[1].cpu_usage,
                "Les processus doivent etre tries par CPU decroissant");
        }
    }

    #[test]
    fn test_syswatcherror_display() {
        let err = SysWatchError::CollectionFailed("test erreur".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test erreur"));
    }

    #[test]
    fn test_tcp_agent_auth_and_commands() {
        let initial = collect_snapshot().expect("collecte initiale");
        let shared = Arc::new(Mutex::new(initial));

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let snap_clone = Arc::clone(&shared);
        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_client(stream, snap_clone);
        });

        let mut client = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let mut reader = BufReader::new(client.try_clone().unwrap());

        let mut token_prompt = String::new();
        reader.read_line(&mut token_prompt).ok();

        client.write_all(b"ENSPD2026\n").unwrap();
        let mut ok_line = String::new();
        reader.read_line(&mut ok_line).ok();
        assert!(ok_line.contains("OK"), "Authentification doit reussir");

        client.write_all(b"help\n").unwrap();
        let mut help_response = String::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if line.trim() == "END" { break; }
                    help_response.push_str(&line);
                }
                Err(_) => break,
            }
        }
        assert!(help_response.contains("cpu"), "Help doit contenir cpu");
        assert!(help_response.contains("exec"), "Help doit contenir exec");
        assert!(help_response.contains("kill"), "Help doit contenir kill");

        client.write_all(b"quit\n").unwrap();
        let mut bye = String::new();
        reader.read_line(&mut bye).ok();
        assert!(bye.contains("BYE"));

        server_thread.join().unwrap();
    }

    #[test]
    fn test_tcp_agent_bad_token() {
        let initial = collect_snapshot().expect("collecte initiale");
        let shared = Arc::new(Mutex::new(initial));

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let snap_clone = Arc::clone(&shared);
        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_client(stream, snap_clone);
        });

        let mut client = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        client.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let mut reader = BufReader::new(client.try_clone().unwrap());

        let mut prompt = String::new();
        reader.read_line(&mut prompt).ok();

        client.write_all(b"WRONG_TOKEN\n").unwrap();
        let mut resp = String::new();
        reader.read_line(&mut resp).ok();
        assert!(resp.contains("UNAUTHORIZED"), "Mauvais token doit etre refuse");

        server_thread.join().unwrap();
    }

    #[test]
    fn test_exec_command_response() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "exec echo hello");
        assert!(result.contains("hello") || result.contains("executee"),
            "exec echo hello devrait renvoyer hello");
    }

    #[test]
    fn test_kill_invalid_pid() {
        let snap = make_test_snapshot();
        let result = format_response(&snap, "kill abc");
        assert!(result.contains("PID invalide"));
    }
}
