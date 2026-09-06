$env:Path = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin;$env:USERPROFILE\.cargo\bin;$env:Path"
& "$env:USERPROFILE\.cargo\bin\cargo.exe" @args

