use std::env::{current_dir, var};
use std::fs;
use std::path::Path;
use std::process::Command;
use winres;

fn main() {
    if var("PROFILE").unwrap() == "release" {
        let mut res = winres::WindowsResource::new();
        res.set_manifest(
            r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
"#,
        ); // require administrator to run
        res.compile().unwrap();
    }

    // integrate rectify11.phase2 into final result
    if !Path::new(
        "Rectify11.Phase2/Rectify11Installer/Rectify11.Phase2/obj/Release/Rectify11.Phase2.exe",
    )
    .exists()
    {
        Command::new("git")
            .current_dir(current_dir().unwrap().as_path())
            .args(["submodule", "update", "--init"])
            .output()
            .unwrap();
        Command::new("msbuild")
            .args([
                "Rectify11.Phase2.sln",
                "/p:Configuration=Release",
                "/p:platform=x64",
            ])
            .current_dir(Path::new("Rectify11.Phase2"))
            .output()
            .unwrap();
    }

    fs::copy(
        Path::new(
            "Rectify11.Phase2/Rectify11Installer/Rectify11.Phase2/obj/Release/Rectify11.Phase2.exe",
        ),
        Path::new("src/Rectify11.Phase2.exe"),
    )
    .expect("Failed to copy Rectify11.Phase2.exe");
}
