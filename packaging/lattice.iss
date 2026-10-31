#define AppName "Lattice"
#ifndef AppVersion
#define AppVersion "0.1.0"
#endif
#ifndef TargetDir
#define TargetDir "..\target\release"
#endif
#define AppPublisher "Lattice"
#define AppExeName "lattice.exe"
#define RootDir ".."

[Setup]
AppId={{D6DC47A4-811A-4A9C-9F7A-5D3A3A5B8C11}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL="https://github.com/cherishtong/lattice"
AppSupportURL="https://github.com/cherishtong/lattice/issues"
AppUpdatesURL="https://github.com/cherishtong/lattice/releases"
DefaultDirName={localappdata}\Programs\Lattice
DefaultGroupName=Lattice
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir={#RootDir}\dist
OutputBaseFilename=Lattice-{#AppVersion}-Setup
LicenseFile={#RootDir}\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#AppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"

[Files]
Source: "{#TargetDir}\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#TargetDir}\lattice-translator.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RootDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RootDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RootDir}\THIRD_PARTY_LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RootDir}\docs\*.md"; DestDir: "{app}\docs"; Flags: ignoreversion

[Icons]
Name: "{group}\Lattice"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\Lattice"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "启动 Lattice"; Flags: nowait postinstall skipifsilent
