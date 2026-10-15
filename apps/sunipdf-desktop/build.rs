//! Ressources Windows : icône de l'exécutable, informations de version, manifeste DPI (per-monitor v2).
fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon/sunipdf.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/icon/sunipdf.ico")
        .set("ProductName", "SuniPDF")
        .set("FileDescription", "SuniPDF — lecteur et éditeur PDF")
        .set("CompanyName", "Navanem")
        .set("LegalCopyright", "© 2026 Emanuel DE ALMEIDA (Navanem). MIT OR Apache-2.0.")
        .set("OriginalFilename", "sunipdf.exe")
        .set_language(0x040C)
        .set_manifest(
            r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="0.1.0.0" processorArchitecture="*" name="Navanem.SuniPDF" type="win32"/>
  <description>SuniPDF</description>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>"#,
        );
    if let Err(e) = res.compile() {
        println!("cargo:warning=ressources Windows non intégrées : {e}");
    }
}
