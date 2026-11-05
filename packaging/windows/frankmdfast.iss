; Inno Setup script for the Windows installer (per-user, no admin rights).
;   iscc /DAppVersion=0.1.0 /DSourceExe=path\to\frankmdfast.exe /DOutputDir=dist packaging\windows\frankmdfast.iss

#ifndef AppVersion
  #error AppVersion must be defined (/DAppVersion=x.y.z)
#endif
#ifndef SourceExe
  #define SourceExe "..\..\target\release\frankmdfast.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif

[Setup]
AppId={{D910776F-5523-4761-B727-A30DF36B9701}
AppName=FrankMDFast
AppVersion={#AppVersion}
AppPublisher=FrankMDFast
AppPublisherURL=https://github.com/akitaonrails/FrankMDFast
DefaultDirName={autopf}\FrankMDFast
DefaultGroupName=FrankMDFast
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=FrankMDFast-{#AppVersion}-windows-x86_64-setup
SetupIconFile=..\..\assets\brand\icon.ico
UninstallDisplayIcon={app}\frankmdfast.exe
LicenseFile=..\..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "frankmdfast.exe"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"
Source: "..\..\README.md"; DestDir: "{app}"

[Icons]
Name: "{autoprograms}\FrankMDFast"; Filename: "{app}\frankmdfast.exe"
Name: "{autodesktop}\FrankMDFast"; Filename: "{app}\frankmdfast.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\frankmdfast.exe"; Description: "{cm:LaunchProgram,FrankMDFast}"; Flags: nowait postinstall skipifsilent
