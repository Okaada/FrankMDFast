; Inno Setup script for the Windows installer (per-user, no admin rights).
;   iscc /DAppVersion=0.1.0 /DSourceExe=path\to\frankmd.exe /DOutputDir=dist packaging\windows\frankmd.iss

#ifndef AppVersion
  #error AppVersion must be defined (/DAppVersion=x.y.z)
#endif
#ifndef SourceExe
  #define SourceExe "..\..\target\release\frankmd.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif

[Setup]
AppId={{C99A7B69-35B0-4732-8767-023C65813E29}
AppName=FrankMD
AppVersion={#AppVersion}
AppPublisher=FrankMD
AppPublisherURL=https://github.com/akitaonrails/FrankMDFast
DefaultDirName={autopf}\FrankMD
DefaultGroupName=FrankMD
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=FrankMD-{#AppVersion}-windows-x86_64-setup
SetupIconFile=..\..\assets\brand\icon.ico
UninstallDisplayIcon={app}\frankmd.exe
LicenseFile=..\..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "frankmd.exe"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"
Source: "..\..\README.md"; DestDir: "{app}"

[Icons]
Name: "{autoprograms}\FrankMD"; Filename: "{app}\frankmd.exe"
Name: "{autodesktop}\FrankMD"; Filename: "{app}\frankmd.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\frankmd.exe"; Description: "{cm:LaunchProgram,FrankMD}"; Flags: nowait postinstall skipifsilent
