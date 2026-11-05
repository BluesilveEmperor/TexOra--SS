; The Windows installer for TeXora, built with Inno Setup 6 by .github/workflows/release.yml:
;   iscc /DAppVersion=1.0.0 /DSourceDir=stage\TeXora-1.0.0-windows-x64 /DOutputDir=dist\1.0.0 installer\texora.iss
; It installs for the current user without asking for administrator rights; people can choose
; "install for all users" instead. Uninstalling removes the program but never their presentations or notes.

#ifndef AppVersion
  #error Pass the version: iscc /DAppVersion=1.0.0 ...
#endif
#ifndef SourceDir
  #error Pass the folder holding TeXora.exe: iscc /DSourceDir=... ...
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

[Setup]
AppId={{6F3C9B2E-4A7D-4C1E-9E58-2B7F0D1A8C34}
AppName=TeXora
AppVersion={#AppVersion}
AppVerName=TeXora {#AppVersion}
AppPublisher=Khalid Alkhaldi
AppPublisherURL=https://github.com/khalidt/TeXora
AppSupportURL=https://github.com/khalidt/TeXora/issues
AppUpdatesURL=https://github.com/khalidt/TeXora/releases
DefaultDirName={autopf}\TeXora
DefaultGroupName=TeXora
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\LICENSE
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\TeXora.exe
UninstallDisplayName=TeXora
OutputDir={#OutputDir}
OutputBaseFilename=TeXora-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "openpdf"; Description: "Add TeXora to ""Open with"" for PDF and .tex files"; GroupDescription: "Files:"

[Files]
Source: "{#SourceDir}\TeXora.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\pdfium.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\CREDITS.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\TeXora"; Filename: "{app}\TeXora.exe"
Name: "{autodesktop}\TeXora"; Filename: "{app}\TeXora.exe"; Tasks: desktopicon

[Registry]
; Offered in "Open with" only; TeXora never takes over the default program for PDFs.
Root: HKA; Subkey: "Software\Classes\Applications\TeXora.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "TeXora"; Flags: uninsdeletekey; Tasks: openpdf
Root: HKA; Subkey: "Software\Classes\Applications\TeXora.exe\shell\open\command"; ValueType: string; ValueData: """{app}\TeXora.exe"" ""%1"""; Tasks: openpdf
Root: HKA; Subkey: "Software\Classes\Applications\TeXora.exe\SupportedTypes"; ValueType: string; ValueName: ".pdf"; ValueData: ""; Tasks: openpdf
Root: HKA; Subkey: "Software\Classes\Applications\TeXora.exe\SupportedTypes"; ValueType: string; ValueName: ".tex"; ValueData: ""; Tasks: openpdf
Root: HKA; Subkey: "Software\Classes\.pdf\OpenWithList\TeXora.exe"; ValueType: none; Flags: uninsdeletekey; Tasks: openpdf
Root: HKA; Subkey: "Software\Classes\.tex\OpenWithList\TeXora.exe"; ValueType: none; Flags: uninsdeletekey; Tasks: openpdf

[Run]
Filename: "{app}\TeXora.exe"; Description: "{cm:LaunchProgram,TeXora}"; Flags: nowait postinstall skipifsilent
