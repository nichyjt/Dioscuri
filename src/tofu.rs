use std::path::PathBuf;
use std::fs;

use native_tls::Certificate;
use x509_parser::prelude::{FromDer, X509Certificate};

/// Checks a certificate given in a TLS stream with the certificate store.
/// If the certificate exists and is not expired, then accept it
/// If the certificate exists and is expired but the PK is different, abort
/// If the certificate !exist, then add it to the certificate store
/// Checks a certificate given in a TLS stream with the certificate store.
/// If the certificate exists and is not expired, then accept it
/// If the certificate exists and is expired but the PK is different, abort
/// If the certificate !exist, then add it to the certificate store
pub fn tofu_handle_certificate(cert: Certificate) -> Result<(), ()> {
    let cert_dir = _tofu_get_cert_dir();
    let cert_der = cert.to_der().unwrap();
    let res = X509Certificate::from_der(&cert_der);
    match res {
        Ok((_, target_cert)) => {
            if !target_cert.validity().is_valid() {
                return Err(())
            }

            let subject_cn = target_cert.subject().iter_common_name().next().unwrap();
            let domain_str = subject_cn.as_str().unwrap();
            let target_cert_path = cert_dir.join(format!("{}.der", domain_str));
            println!("Certificate for domain: {}", domain_str);
            // Search cert store
            if target_cert_path.exists() {
                println!("Certificate found in certificate store!");

                let src_cert_der = fs::read(&target_cert_path).unwrap();
                let (_, src_cert) = X509Certificate::from_der(&src_cert_der).unwrap();
                
                // If the source certificate is valid, check the public keys
                if src_cert.validity().is_valid() {
                    let src_pub_key = src_cert.public_key();
                    let target_pub_key = target_cert.public_key();

                    if src_pub_key == target_pub_key {
                        println!("Certificate is valid and public keys match.");
                        return Ok(()); // Accept if the certificate is valid and public keys match
                    } else {
                        println!("Public keys do not match. Aborting trust.");
                        return Err(()); // Abort if the public keys do not match
                    }
                } else {
                    // Source certificate is expired, update if public keys match
                    let src_pub_key = src_cert.public_key();
                    let target_pub_key = target_cert.public_key();

                    if src_pub_key == target_pub_key {
                        println!("Source certificate expired but public keys match. Updating certificate.");
                        fs::write(&target_cert_path, &cert_der).unwrap();
                        return Ok(());
                    } else {
                        println!("Source certificate expired but public keys do not match. Abort!");
                        return Err(());
                    }
                }
            }else{
                println!("Certificate does not exist in the store. Adding to trust store (TOFU)...");
                fs::create_dir_all(&cert_dir).unwrap();
                fs::write(&target_cert_path, &cert_der).unwrap();
                println!("New certificate for {} added to trust store.", domain_str);
                return Ok(()); // Add and accept the new certificate
            }
        },
        Err(_) => {
            println!("Error: Certificate from server is invalid.");
            Err(()) // Return error if certificate parsing failed
        }
    }
}

/// Ensures that a certificate store directory cert/ is made in ~/.dioscuri (or the equivalent home directory)
fn _tofu_setup_directory() -> Result<PathBuf, std::io::Error> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory does not exist?")
    })?;

    let dioscuri_dir = home_dir.join(".dioscuri/cert");

    if !dioscuri_dir.exists() {
        fs::create_dir_all(&dioscuri_dir)?;
        println!("Creating directory: {:?}", dioscuri_dir);
    }
    Ok(dioscuri_dir)
}

fn _tofu_get_cert_dir() -> PathBuf {
    let _ = _tofu_setup_directory();
    let home = dirs::home_dir().unwrap();
    return home.join(".dioscuri/cert");
}
