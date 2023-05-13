#![feature(exit_status_error)]

use std::env::var;
use std::fmt::Debug;
use std::fmt::Display;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::{fs, process};

use registry::{Data, Hive, Security};
use utfx::U16CString;

fn main() {
    println!(
        "This program is not maintained nor created by the Rectify11 team. \
        If you experience issues with this program, create an issue on the Github page. \
        https://github.com/xverge/rectify-uninstaller"
    );
    println!("If you do not wish to continue, close the application now.");
    pause();
    let rectify_key = match Hive::LocalMachine.open(r"SOFTWARE\Rectify11", Security::AllAccess) {
        Ok(key) => key,
        Err(e) => {
            exit_recitfy11(Some(Box::new(e)));
            panic!()
        }
    };
    if matches!(
        rectify_key
            .value("IsInstalled")
            .expect_pause("Failed to open value IsInstalled"),
        Data::U32(0)
    ) {
        exit_recitfy11(None)
    }
    let Data::MultiString(pending_files) = rectify_key.value("PendingFiles").expect_pause("Failed to open value PendingFiles") else { panic!("This should never happen. If you recieve this error, you have a broken or corrupted Rectify11 install."); };
    let Data::MultiString(pending_files_x86) = rectify_key.value("x86PendingFiles").expect_pause("Failed to open value x86PendingFiles") else { panic!("This should never happen. If you recieve this error, you have a broken or corrupted Rectify11 install."); };
    let mut uninstall_files: Vec<U16CString> = Vec::new();
    for x in pending_files {
        uninstall_files.push(x);
    }
    for x in pending_files_x86 {
        uninstall_files.push(x);
    }
    rectify_key
        .set_value("UninstallFiles", &Data::MultiString(uninstall_files))
        .expect_pause("Failed to write to UninstallFiles registry");
    let phase2_path = format!(
        "{}/Rectify11.Phase2.exe",
        var("TEMP").expect_pause("Failed to read TEMP environment variable")
    );
    let mut phase2 = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(PathBuf::from(phase2_path.clone()))
        .unwrap();
    phase2
        .write_all(include_bytes!("Rectify11.Phase2.exe"))
        .unwrap();
    phase2.flush().unwrap();
    drop(phase2); // force drop so that it's no longer "being used by another (same) process"
    println!("Removing patched files...");
    command(phase2_path.as_str(), "/uninstall", temp().as_path())
        .exit_ok()
        .expect_pause("Rectify11.Phase2.exe failed to execute.");
    println!("Finished removing patched files!");
    rectify_key
        .set_value("IsInstalled", &Data::U32(0))
        .expect_pause("Failed to set Rectify11 IsInstalled registry key");
    println!("Removing certificate key..");
    match unsafe { std::panic::catch_unwind(|| cert::delete_cert()) } {
        Err(e) => {
            println!();
            println!();
            eprintln!("{:?}", e);
            eprintln!("Failed to delete certificate key.");
            eprintln!("If you are running a Rectify11 actions build from after May 6th, 2023,");
            eprintln!("after uninstallation you will have to delete the Rectify11 certificate");
            eprintln!("using certlm.msc.");
            println!("If you are running an older build, or are using Release Preview 1,2, and 3,");
            println!("you can safely ignore this message.");
            pause();
        }
        _ => {
            println!("Successfully removed certificate key!");
        }
    };
    println!("Removing the remainder of Rectify11 (your shell will close during this process)");
    kill_all();
    delete_tasks();
    del_dir("MicaForEveryone");
    del_dir("nilesoft");
    del_dir("Rectify11");
    del_dir(r"web\Wallpaper\Rectified");
    match Hive::LocalMachine.open(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Rectify11",
        Security::AllAccess,
    ) {
        Ok(key) => key.delete_self(true).unwrap_pause(),
        _ => {}
    };
    println!("Successfully uninstalled Rectify11.");
    println!("Continue to restart your computer.");
    pause();
    command("shutdown.exe", "-r -t 0", temp().as_path());
}

fn exit_recitfy11(error: Option<Box<dyn std::error::Error>>) {
    eprintln!("Failed to verify you are running Recitfy11. You may not have it installed.");
    match error {
        Some(e) => eprintln!("{}", e),
        _ => {}
    }
    pause();
    process::exit(1);
}

fn command(app: &str, args: &str, dir: &Path) -> ExitStatus {
    let split: Vec<&str> = args.split(" ").collect();
    Command::new(app)
        .args(split)
        .current_dir(dir)
        .status()
        .expect_pause_format(format!(
            "Failed to run command {}, directory {}, args {}",
            app,
            dir.display(),
            args
        ))
}

fn kill_all() {
    fn taskkill(app: &str) {
        command(
            "taskkill",
            format!("/f /im {}", app).as_str(),
            temp().as_path(),
        );
    }
    taskkill("MicaForEveryone.exe");
    taskkill("ExplorerFrame.exe");
    taskkill("AccentColorizer.exe");
    taskkill("AccentColorizerEleven.exe");
    taskkill("explorer.exe");
}

fn delete_tasks() {
    fn del(task: &str) {
        command(
            "schtasks",
            format!("/delete /tn {} /f", task).as_str(),
            temp().as_path(),
        );
    }
    del("mfe");
    del("micafix");
    del("gadgets");
    command("sc", "delete RectifySounds", temp().as_path());
}

fn del_dir(dir: &str) {
    if Path::new(
        format!(
            r"{}\{}",
            var("WINDIR").expect_pause("Failed to read WINDIR environment variable"),
            dir
        )
        .as_str(),
    )
    .exists()
    {
        fs::remove_dir_all(format!(
            "{}/{}",
            var("WINDIR").expect_pause("Failed to read WINDIR environment variable"),
            dir
        ))
        .expect_pause_format(format!("Failed to delete {} directory", dir));
    }
}

fn pause() {
    Command::new("cmd.exe")
        .arg("/c")
        .arg("pause")
        .status()
        .expect("Failed to pause");
}

trait WaitBefore<T, E> {
    fn expect_pause_format(self, str: String) -> T;
    fn expect_pause(self, str: &str) -> T;
    fn unwrap_pause(self) -> T;
}

impl<T, E: Debug + Display> WaitBefore<T, E> for Result<T, E> {
    fn expect_pause_format(self, str: String) -> T {
        self.expect_pause(str.as_str())
    }

    fn expect_pause(self, str: &str) -> T {
        match self {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error message: {}, error: {}", str, e);
                pause();
                process::exit(1);
            }
        }
    }

    fn unwrap_pause(self) -> T {
        match self {
            Ok(object) => object,
            Err(e) => {
                eprintln!("{}", e);
                pause();
                process::exit(1);
            }
        }
    }
}

fn temp() -> PathBuf {
    PathBuf::from(var("TEMP").expect_pause("Failed to read TEMP environment variable"))
}

// based off Rectify11Installer Signer.handleDeleteCommand
// https://github.com/MishaProductions/Rectify11Installer/blob/master/Rectify11Installer/Core/Signing/Signer.cs
mod cert {
    use std::ffi::c_void;
    use std::ptr::null_mut;
    use windows_sys::Win32::Security::Cryptography::*;
    pub(crate) unsafe fn delete_cert() {
        let machine = true;
        let name = "Microsoft Windows";
        let mut cert_store = open_store(name, machine);
        let cert = find_cert(name, true, cert_store);
        if !CertDeleteCertificateFromStore(cert) > 0 {
            panic!("Failed to delete R11 certificate")
        }
        CertCloseStore(cert_store, 0);

        // Open the root store
        if {
            cert_store = open_store("Root", machine);
            cert_store == null_mut()
        } {
            panic!("Failed to open root store");
        }

        let cert2 = find_cert(
            format!("{} Certificate Authority", name).as_str(),
            true,
            cert_store,
        );
        if !CertDeleteCertificateFromStore(cert2) > 0 {
            panic!("Failed to delete R11 certificate")
        }
        CertCloseStore(cert_store, 0);
    }

    unsafe fn find_cert(
        cert_name: &str,
        has_private_key: bool,
        cert_store: HCERTSTORE,
    ) -> *mut CERT_CONTEXT {
        let mut context: *mut CERT_CONTEXT = null_mut();
        while {
            context = CertEnumCertificatesInStore(cert_store, context);
            context.as_ref().is_none()
        } {
            if !has_private_key || super::cert::has_private_key(context) {
                let mut buffer: Vec<u16> = vec![0; 256 as usize];
                if CertGetNameStringW(
                    context,
                    CERT_NAME_FRIENDLY_DISPLAY_TYPE,
                    0,
                    0 as *const c_void,
                    buffer.as_mut_ptr(),
                    256,
                ) != 0
                {
                    let str = String::from_utf16_lossy(&*buffer);
                    println!("{}", str);
                    if str.replace("\0", "").as_str() == cert_name {
                        return context;
                    }
                }
            }
        }
        panic!("The certificate does not exist"); // will be caught
    }

    unsafe fn has_private_key(cert: *mut CERT_CONTEXT) -> bool {
        CryptFindCertificateKeyProvInfo(
            cert,
            CRYPT_ACQUIRE_ALLOW_NCRYPT_KEY_FLAG,
            0 as *const c_void,
        ) > 0
    }

    unsafe fn open_store(name: &str, machine: bool) -> HCERTSTORE {
        let dw_flags = if machine {
            CERT_SYSTEM_STORE_LOCAL_MACHINE_ID
        } else {
            CERT_SYSTEM_STORE_CURRENT_USER_ID
        };
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            dw_flags,
            name.as_ptr() as *const c_void,
        )
    }
}
