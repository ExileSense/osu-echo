use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

const TLS_SUBDOMAINS: &[&str] =
    &["localhost", "c.localhost", "c4.localhost", "c5.localhost", "c6.localhost", "ce.localhost", "osu.localhost", "b.localhost", "a.localhost", "assets.localhost"];

const CURRENT_CERT_VERSION: &str = "v2_with_b_localhost";

pub struct TlsSetupPaths {
    pub cert_pem: PathBuf,
    pub key_pem: PathBuf,
    pub cert_der: PathBuf,
}

pub fn get_or_create_certificates(data_dir: &Path) -> Result<TlsSetupPaths, Box<dyn std::error::Error>> {
    let tls_dir = data_dir.join("local-tls");
    std::fs::create_dir_all(&tls_dir)?;

    let cert_pem = tls_dir.join("localhost.pem");
    let key_pem = tls_dir.join("localhost-key.pem");
    let cert_der = tls_dir.join("localhost.cer");
    let cert_ver = tls_dir.join("cert_version.txt");

    let is_up_to_date = cert_ver.exists() && std::fs::read_to_string(&cert_ver).unwrap_or_default() == CURRENT_CERT_VERSION;

    if cert_pem.exists() && key_pem.exists() && cert_der.exists() && is_up_to_date {
        return Ok(TlsSetupPaths { cert_pem, key_pem, cert_der });
    }

    crate::logger::info("Generating self-signed TLS certificates for localhost (including b.localhost)...");

    let mut params = CertificateParams::default();
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Local osu localhost");
    params.distinguished_name = dn;

    let mut sans = Vec::new();
    for sub in TLS_SUBDOMAINS {
        sans.push(SanType::DnsName((*sub).try_into()?));
    }
    sans.push(SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))));
    params.subject_alt_names = sans;

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    let cert_pem_data = cert.pem();
    let cert_der_data = cert.der().to_vec();
    let key_pem_data = key_pair.serialize_pem();

    std::fs::write(&cert_pem, cert_pem_data)?;
    std::fs::write(&key_pem, key_pem_data)?;
    std::fs::write(&cert_der, &cert_der_data)?;
    let _ = std::fs::write(&cert_ver, CURRENT_CERT_VERSION);

    crate::logger::success(&format!("TLS certificates generated in {}", tls_dir.display()));
    let _ = install_windows_trust(&cert_der);

    Ok(TlsSetupPaths { cert_pem, key_pem, cert_der })
}

pub fn install_windows_trust(der_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        crate::logger::info("Registering certificate into Windows User Root Trust store...");
        let output = std::process::Command::new("certutil").args(["-user", "-addstore", "Root", &der_path.to_string_lossy()]).output()?;

        if output.status.success() {
            crate::logger::success("Local certificate trusted by Windows successfully.");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            crate::logger::warn(&format!("certutil returned: {}", stderr.trim()));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = der_path;
    }
    Ok(())
}

pub fn hosts_ready() -> bool {
    #[cfg(target_os = "windows")]
    {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let hosts_path = std::path::Path::new(&system_root).join("System32\\drivers\\etc\\hosts");
        let content = std::fs::read_to_string(hosts_path).unwrap_or_default();
        return ["osu.localhost", "c.localhost", "cho.localhost", "a.localhost", "assets.localhost", "b.localhost"].iter().all(|d| content.contains(d));
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

pub fn setup_windows_hosts() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        use std::io::Write;

        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        let hosts_path = std::path::Path::new(&system_root).join("System32\\drivers\\etc\\hosts");

        if !hosts_path.exists() {
            crate::logger::warn(&format!("Hosts file not found at {}", hosts_path.display()));
            return Ok(());
        }

        let content = std::fs::read_to_string(&hosts_path).unwrap_or_default();
        let required = ["osu.localhost", "c.localhost", "cho.localhost", "a.localhost", "assets.localhost", "b.localhost"];
        let all_present = required.iter().all(|domain| content.contains(domain));

        if all_present {
            crate::logger::info("Windows hosts file already contains localhost subdomains.");
            return Ok(());
        }

        crate::logger::info("Configuring Windows hosts file for -devserver localhost subdomains...");
        let hosts_line = "\r\n127.0.0.1 osu.localhost c.localhost cho.localhost c1.localhost c2.localhost c3.localhost c4.localhost c5.localhost c6.localhost ce.localhost a.localhost assets.localhost b.localhost # osu-echo\r\n";

        let direct_success = std::fs::OpenOptions::new()
            .append(true)
            .open(&hosts_path)
            .and_then(|mut f| f.write_all(hosts_line.as_bytes()))
            .is_ok();

        if direct_success {
            crate::logger::success("Added localhost subdomains to Windows hosts file.");
            let _ = std::process::Command::new("ipconfig").arg("/flushdns").output();
            crate::logger::info("If the website does not load, reload the page with CTRL + F5.");
            return Ok(());
        }

        crate::logger::warn("ADMINISTRATOR PERMISSION NEEDED: osu-echo must add osu.localhost to your Windows hosts file.");
        crate::logger::warn("Please click 'Yes' on the Windows prompt (or close this window and run osu-echo as administrator).");
        let ps_script = format!(
            "Add-Content -Path '{}\\System32\\drivers\\etc\\hosts' -Value '`r`n127.0.0.1 osu.localhost c.localhost cho.localhost c1.localhost c2.localhost c3.localhost c4.localhost c5.localhost c6.localhost ce.localhost a.localhost assets.localhost b.localhost # osu-echo'",
            system_root.replace('\'', "''")
        );

        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile', '-Command', \"{}\"", ps_script.replace('"', "\\\"")),
            ])
            .output();

        let updated_content = std::fs::read_to_string(&hosts_path).unwrap_or_default();
        if required.iter().all(|domain| updated_content.contains(domain)) {
            crate::logger::success("Added localhost subdomains to Windows hosts file successfully.");
            let _ = std::process::Command::new("ipconfig").arg("/flushdns").output();
            crate::logger::info("If the website does not load, reload the page with CTRL + F5.");
        } else {
            crate::logger::warn("Could not automatically update Windows hosts file (elevation was cancelled or failed).");
            crate::logger::warn("osu.localhost will not open until this is done: close osu-echo and start it again as administrator (right-click > Run as administrator).");
            crate::logger::warn("Please manually add the following line to C:\\Windows\\System32\\drivers\\etc\\hosts as Administrator:");
            crate::logger::warn("  127.0.0.1 osu.localhost c.localhost cho.localhost c1.localhost c2.localhost c3.localhost c4.localhost c5.localhost c6.localhost ce.localhost a.localhost assets.localhost b.localhost");
        }
    }

    Ok(())
}

pub fn create_tls_acceptor(cert_path: &Path, key_path: &Path) -> Result<TlsAcceptor, Box<dyn std::error::Error>> {
    let cert_file = std::fs::File::open(cert_path)?;
    let mut cert_reader = std::io::BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

    let key_file = std::fs::File::open(key_path)?;
    let mut key_reader = std::io::BufReader::new(key_file);
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_reader)?.ok_or("No private key found in key file")?;

    let config = ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key)?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

pub fn describe_client_hello(buf: &[u8]) -> String {
    let hex: String = buf.iter().take(24).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
    if buf.len() < 44 || buf[0] != 0x16 || buf[5] != 0x01 {
        return format!("not a TLS ClientHello ({} bytes, starts {})", buf.len(), hex);
    }
    let rec_ver = format!("{}.{}", buf[1], buf[2]);
    let hello_ver = format!("{}.{}", buf[9], buf[10]);
    let mut pos = 43;
    let sid = *buf.get(pos).unwrap_or(&0) as usize;
    pos += 1 + sid;
    let cs_len = match buf.get(pos..pos + 2) {
        Some(b) => ((b[0] as usize) << 8) | b[1] as usize,
        None => return format!("TLS ClientHello rec={} hello={} (truncated)", rec_ver, hello_ver),
    };
    let suites: Vec<String> = buf
        .get(pos + 2..pos + 2 + cs_len)
        .unwrap_or(&[])
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
        .collect();
    pos += 2 + cs_len;
    let comp = *buf.get(pos).unwrap_or(&0) as usize;
    pos += 1 + comp;
    let mut exts: Vec<String> = Vec::new();
    if let Some(b) = buf.get(pos..pos + 2) {
        let total = ((b[0] as usize) << 8) | b[1] as usize;
        let mut p = pos + 2;
        let end = (p + total).min(buf.len());
        while p + 4 <= end {
            let t = ((buf[p] as u16) << 8) | buf[p + 1] as u16;
            let l = ((buf[p + 2] as usize) << 8) | buf[p + 3] as usize;
            exts.push(format!("{}", t));
            p += 4 + l;
        }
    }
    format!("TLS ClientHello rec={} hello={} suites=[{}] extensions=[{}]", rec_ver, hello_ver, suites.join(","), exts.join(","))
}

pub fn hello_needs_legacy_tls(buf: &[u8]) -> bool {
    buf.len() >= 11 && buf[0] == 0x16 && buf[5] == 0x01 && buf[9] == 3 && buf[10] < 3
}

pub fn create_legacy_acceptor(cert_path: &Path, key_path: &Path) -> Result<tokio_native_tls::TlsAcceptor, Box<dyn std::error::Error>> {
    let mut cert_reader = std::io::BufReader::new(std::fs::File::open(cert_path)?);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;
    let leaf = certs.first().ok_or("No certificate found in cert file")?;

    let mut key_reader = std::io::BufReader::new(std::fs::File::open(key_path)?);
    let key = rustls_pemfile::private_key(&mut key_reader)?.ok_or("No private key found in key file")?;
    let key_der: Vec<u8> = match key {
        PrivateKeyDer::Pkcs8(k) => k.secret_pkcs8_der().to_vec(),
        _ => return Err("Legacy TLS needs a PKCS#8 private key".into()),
    };

    let pfx = p12::PFX::new(leaf.as_ref(), &key_der, None, "osu-echo", "osu-echo").ok_or("Could not build the PKCS#12 bundle")?;
    let identity = native_tls::Identity::from_pkcs12(&pfx.to_der(), "osu-echo")?;

    let mut builder = native_tls::TlsAcceptor::builder(identity);
    builder.min_protocol_version(Some(native_tls::Protocol::Tlsv10));
    Ok(tokio_native_tls::TlsAcceptor::from(builder.build()?))
}
