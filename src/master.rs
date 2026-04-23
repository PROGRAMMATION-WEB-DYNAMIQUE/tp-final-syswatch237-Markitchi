// src/master.rs
// Interface maitre SysWatch -- tourne sur le PC du professeur

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

const AUTH_TOKEN: &str = "ENSPD2026";
const TCP_PORT: u16 = 7878;

struct AgentSession {
    name: String,
    #[allow(dead_code)]
    ip: String,
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl AgentSession {
    fn connect(name: &str, ip: &str) -> Result<Self, String> {
        let addr = format!("{}:{}", ip, TCP_PORT);
        let stream = TcpStream::connect_timeout(
            &addr.parse().map_err(|e| format!("{}", e))?,
            Duration::from_secs(3),
        )
        .map_err(|e| format!("Connexion refusee: {}", e))?;

        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

        let mut session = AgentSession {
            name: name.to_string(),
            ip: ip.to_string(),
            stream: stream.try_clone().unwrap(),
            reader: BufReader::new(stream),
        };

        // Lire le prompt TOKEN
        let token_line = session.read_line()?;
        if !token_line.contains("TOKEN") {
            return Err(format!("Prompt inattendu: {}", token_line.trim()));
        }

        // Envoyer le token
        session.send(AUTH_TOKEN)?;
        let resp = session.read_line()?;
        if resp.trim() != "OK" {
            return Err("Token refuse par l'agent".to_string());
        }

        Ok(session)
    }

    fn send(&mut self, cmd: &str) -> Result<(), String> {
        self.stream
            .write_all(format!("{}\n", cmd).as_bytes())
            .map_err(|e| format!("Erreur envoi: {}", e))
    }

    fn read_line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| format!("Erreur lecture: {}", e))?;
        Ok(line)
    }

    fn read_until_end(&mut self) -> Result<String, String> {
        let mut result = String::new();
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if line.trim() == "END" {
                        break;
                    }
                    result.push_str(&line);
                }
                Err(_) => break,
            }
        }
        Ok(result)
    }

    fn run_command(&mut self, cmd: &str) -> String {
        match self.send(cmd) {
            Err(e) => format!("Erreur: {}", e),
            Ok(_) => self.read_until_end().unwrap_or_else(|e| format!("Erreur lecture: {}", e)),
        }
    }

    fn is_alive(&mut self) -> bool {
        let mut buf = [0u8; 0];
        match self.stream.set_nonblocking(true) {
            Ok(_) => {
                let result = match self.stream.peek(&mut buf) {
                    Ok(_) => true,
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
                    Err(_) => false,
                };
                self.stream.set_nonblocking(false).ok();
                result
            }
            Err(_) => false,
        }
    }
}

// Detecter l'IP locale de la machine
fn get_local_ip() -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    match addr.ip() {
        std::net::IpAddr::V4(ip) => Some(ip),
        _ => None,
    }
}

// Tester si une IP a un agent SysWatch actif
fn probe_agent(ip: Ipv4Addr) -> Option<(String, String)> {
    let addr_str = format!("{}:{}", ip, TCP_PORT);
    let sock_addr: std::net::SocketAddr = addr_str.parse().ok()?;

    let stream = TcpStream::connect_timeout(&sock_addr, Duration::from_millis(400)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();

    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;

    if line.contains("TOKEN") {
        // C'est un agent SysWatch, envoyer le token pour recuperer le hostname
        let mut stream = stream;
        stream.write_all(format!("{}\n", AUTH_TOKEN).as_bytes()).ok()?;

        let mut resp = String::new();
        reader.read_line(&mut resp).ok()?;

        if resp.trim() == "OK" {
            // Demander le hostname via exec
            stream.write_all(b"exec hostname\n").ok()?;
            let mut hostname = String::new();
            loop {
                let mut l = String::new();
                match reader.read_line(&mut l) {
                    Ok(0) => break,
                    Ok(_) => {
                        if l.trim() == "END" { break; }
                        hostname.push_str(&l);
                    }
                    Err(_) => break,
                }
            }
            stream.write_all(b"quit\n").ok();
            let name = hostname.trim().to_string();
            let name = if name.is_empty() { format!("Agent-{}", ip) } else { name };
            return Some((name, ip.to_string()));
        }
    }
    None
}

// Scanner le sous-reseau local pour trouver les agents SysWatch
fn discover_agents() -> HashMap<String, String> {
    let discovered: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    let local_ip = match get_local_ip() {
        Some(ip) => ip,
        None => {
            eprintln!("  Impossible de detecter l'IP locale.");
            return HashMap::new();
        }
    };

    let octets = local_ip.octets();
    println!("  IP locale: {}", local_ip);
    println!(
        "  Scan du sous-reseau {}.{}.{}.0/24 sur le port {}...",
        octets[0], octets[1], octets[2], TCP_PORT
    );

    let mut handles = vec![];

    for i in 1..=254u8 {
        let ip = Ipv4Addr::new(octets[0], octets[1], octets[2], i);
        let discovered = Arc::clone(&discovered);

        let handle = thread::spawn(move || {
            if let Some((name, ip_str)) = probe_agent(ip) {
                let mut map = discovered.lock().unwrap();
                println!("  [+] Decouvert: {} ({})", name, ip_str);
                map.insert(name, ip_str);
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().ok();
    }

    let result = discovered.lock().unwrap().clone();

    if result.is_empty() {
        println!("  Aucun agent detecte sur le sous-reseau.");
        println!("  Conseil: utilisez 'add <nom> <ip>' pour ajouter manuellement.");
    } else {
        println!("  {} agent(s) trouve(s).", result.len());
    }

    result
}

fn connect_to(name: &str, ip: &str) -> Option<AgentSession> {
    print!("  Connexion a {} ({})... ", name, ip);
    std::io::stdout().flush().ok();
    match AgentSession::connect(name, ip) {
        Ok(s) => {
            println!("[OK]");
            Some(s)
        }
        Err(e) => {
            println!("[ECHEC] {}", e);
            None
        }
    }
}

fn print_menu() {
    println!("\n+=============================================+");
    println!("|       SYSWATCH MASTER -- ENSPD 2026         |");
    println!("+==============================================+");
    println!("|  scan             - scanner le reseau        |");
    println!("|  list             - afficher agents connus   |");
    println!("|  add <nom> <ip>   - ajouter manuellement     |");
    println!("|  select <nom>     - cibler une machine       |");
    println!("|  all <cmd>        - envoyer cmd a toutes     |");
    println!("+==============================================+");
    println!("|  Commandes agent (apres select) :            |");
    println!("|  cpu / mem / ps / all                        |");
    println!("|  exec <cmd>   - executer commande shell      |");
    println!("|  kill <pid>   - tuer un processus            |");
    println!("|  lock         - verrouiller le poste         |");
    println!("|  users        - lister les utilisateurs      |");
    println!("|  msg <texte>  - afficher message             |");
    println!("|  install <pkg> - installer un logiciel       |");
    println!("|  shutdown     - eteindre la machine          |");
    println!("|  reboot       - redemarrer                   |");
    println!("|  abort        - annuler extinction           |");
    println!("+==============================================+");
    println!("|  disconnect   - fermer la session active     |");
    println!("|  help         - afficher ce menu             |");
    println!("|  quit         - quitter le master            |");
    println!("+=============================================+");
}

fn main() {
    print_menu();

    let mut machines: HashMap<String, String> = HashMap::new();
    let mut active_session: Option<AgentSession> = None;
    let mut selected_name: Option<String> = None;
    let stdin = std::io::stdin();

    loop {
        let prompt = match &selected_name {
            Some(name) => format!("[master@{}]> ", name),
            None => "[master]> ".to_string(),
        };
        print!("{}", prompt);
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        stdin.lock().read_line(&mut input).unwrap();
        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        match input.as_str() {
            "quit" | "exit" => {
                if let Some(mut session) = active_session.take() {
                    session.send("quit").ok();
                }
                println!("Au revoir.");
                break;
            }

            "help" => print_menu(),

            "scan" => {
                println!("\nScan du reseau...");
                let discovered = discover_agents();
                for (name, ip) in &discovered {
                    machines.insert(name.clone(), ip.clone());
                }
                if !machines.is_empty() {
                    println!("\nMachines connues:");
                    for (name, ip) in &machines {
                        println!("  {} ({})", name, ip);
                    }
                }
            }

            "list" => {
                if machines.is_empty() {
                    println!("Aucune machine connue. Lance 'scan' d'abord.");
                } else {
                    println!("\nMachines connues:");
                    for (name, ip) in &machines {
                        let marker = match &selected_name {
                            Some(sel) if sel == name => " <-- actif",
                            _ => "",
                        };
                        println!("  {} ({}){}", name, ip, marker);
                    }
                }
            }

            "disconnect" => {
                if let Some(mut session) = active_session.take() {
                    session.send("quit").ok();
                    println!("Session fermee avec {}.", session.name);
                }
                selected_name = None;
            }

            _ if input.starts_with("add ") => {
                let args: Vec<&str> = input[4..].trim().splitn(2, ' ').collect();
                if args.len() == 2 {
                    let name = args[0].to_string();
                    let ip = args[1].to_string();
                    println!("Machine ajoutee: {} ({})", name, ip);
                    machines.insert(name, ip);
                } else {
                    println!("Usage: add <nom> <ip>  (ex: add PC-01 192.168.1.50)");
                }
            }

            _ if input.starts_with("select ") => {
                let name = input[7..].trim().to_string();
                if let Some(ip) = machines.get(&name) {
                    if let Some(mut old) = active_session.take() {
                        old.send("quit").ok();
                    }

                    let ip = ip.clone();
                    match connect_to(&name, &ip) {
                        Some(session) => {
                            selected_name = Some(name.clone());
                            active_session = Some(session);
                        }
                        None => {
                            selected_name = None;
                            println!("Impossible de se connecter a {}.", name);
                        }
                    }
                } else {
                    println!(
                        "Machine inconnue: '{}'. Lance 'scan' ou 'add <nom> <ip>'.",
                        name
                    );
                }
            }

            _ if input.starts_with("all ") => {
                let cmd = input[4..].trim().to_string();
                if machines.is_empty() {
                    println!("Aucune machine connue. Lance 'scan' d'abord.");
                    continue;
                }

                println!("Envoi de '{}' a toutes les machines...\n", cmd);

                let machine_list: Vec<(String, String)> = machines
                    .iter()
                    .map(|(n, i)| (n.clone(), i.clone()))
                    .collect();

                for (name, ip) in &machine_list {
                    print!("  {} -- ", name);
                    std::io::stdout().flush().unwrap();
                    match AgentSession::connect(name, ip) {
                        Ok(mut session) => {
                            let response = session.run_command(&cmd);
                            let first_line = response.lines().next().unwrap_or("(vide)");
                            println!("{}", first_line);
                            session.send("quit").ok();
                        }
                        Err(e) => println!("[HORS LIGNE] {}", e),
                    }
                }
            }

            cmd => {
                match &selected_name {
                    None => {
                        println!("Aucune machine selectionnee. Utilise 'scan' puis 'select <nom>'.");
                    }
                    Some(name) => {
                        let needs_reconnect = match &mut active_session {
                            Some(session) => !session.is_alive(),
                            None => true,
                        };

                        if needs_reconnect {
                            println!("  (reconnexion...)");
                            let ip = machines[name].clone();
                            if let Some(mut old) = active_session.take() {
                                old.send("quit").ok();
                            }
                            active_session = connect_to(name, &ip);
                            if active_session.is_none() {
                                println!("Impossible de se reconnecter a {}.", name);
                                selected_name = None;
                                continue;
                            }
                        }

                        if let Some(session) = active_session.as_mut() {
                            let response = session.run_command(cmd);
                            println!("{}", response);
                        }
                    }
                }
            }
        }
    }
}
