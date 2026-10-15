; The Windows installer (Inno Setup 6), compiled by packaging/windows/package.ps1, which defines:
;   AppVersion   the version
;   SourceDir    the staged app: nettai.exe, ffmpeg.exe, content\, LICENSE.txt, THIRD-PARTY-NOTICES.txt,
;                ffmpeg-NOTICE.txt
;   OutputDir    where the installer is written
;   IconFile     (optional) the .ico packaging/icons.sh makes
; For the player alone by default (no administrator: %LOCALAPPDATA%\Programs\nettai); the installer offers to
; install for everyone. The uninstaller leaves the player's data folder (%APPDATA%\nettai: builds, replays,
; settings, the extracted packs) as it is.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{41CA549D-FE36-465B-ACA7-DCA9A944463E}
AppName=nettai
AppVersion={#AppVersion}
AppVerName=nettai {#AppVersion}
AppPublisher=nettai
AppPublisherURL=https://github.com/bigfarts/nettai
AppSupportURL=https://github.com/bigfarts/nettai/issues
DefaultDirName={autopf}\nettai
DefaultGroupName=nettai
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=nettai-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\nettai.exe
UninstallDisplayName=nettai
#ifdef IconFile
SetupIconFile={#IconFile}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[InstallDelete]
; (An update's content replaces the last one's whole: a module it no longer has isn't left behind.)
Type: filesandordirs; Name: "{app}\content"

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\nettai"; Filename: "{app}\nettai.exe"
Name: "{autodesktop}\nettai"; Filename: "{app}\nettai.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\nettai.exe"; Description: "{cm:LaunchProgram,nettai}"; Flags: nowait postinstall skipifsilent
