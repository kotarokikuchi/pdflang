; Inno Setup script for the Windows installer.
;
; Driven entirely by /D flags from the release workflow, so nothing here needs
; editing per release:
;   iscc /DAppVersion=0.8.1 /DSourceDir=dist\pdfl-0.8.1-windows-x64 /DOutputName=... pdfl.iss

[Setup]
AppName=pdfl
AppVersion={#AppVersion}
AppPublisher=Kotaro Kikuchi
AppPublisherURL=https://github.com/kotarokikuchi/pdflang
DefaultDirName={autopf}\pdfl
DefaultGroupName=pdfl
LicenseFile={#SourceDir}\LICENSE
OutputBaseFilename={#OutputName}
OutputDir=.
Compression=lzma2
SolidCompression=yes
; pdfium.dll is x64-only, so refuse to install where it could not run.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Per-machine when elevated, per-user otherwise — no forced UAC prompt.
PrivilegesRequiredOverridesAllowed=dialog
ChangesEnvironment=yes
WizardStyle=modern

[Files]
Source: "{#SourceDir}\pdfl.exe";      DestDir: "{app}"
Source: "{#SourceDir}\README.md";     DestDir: "{app}"
Source: "{#SourceDir}\LICENSE";       DestDir: "{app}"
; pdfium.dll sits in pdfium\bin: the binary looks for <exe dir>\pdfium\bin.
Source: "{#SourceDir}\pdfium\*";      DestDir: "{app}\pdfium";   Flags: recursesubdirs
Source: "{#SourceDir}\examples\*";    DestDir: "{app}\examples"; Flags: recursesubdirs

[Tasks]
Name: addtopath; Description: "Add pdfl to the PATH (recommended for a command-line tool)"

[Registry]
; The environment key differs between a per-machine and a per-user install.
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"; \
    ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; \
    Check: IsAdminInstallMode and NeedsAddPath(ExpandConstant('{app}')); Tasks: addtopath
Root: HKCU; Subkey: "Environment"; \
    ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; \
    Check: (not IsAdminInstallMode) and NeedsAddPath(ExpandConstant('{app}')); Tasks: addtopath

[Icons]
Name: "{group}\pdfl (command prompt)"; Filename: "{cmd}"; Parameters: "/K cd /d ""{app}"""

[Code]
{ Appending blindly would grow PATH by one copy per reinstall. }
function NeedsAddPath(Dir: string): boolean;
var
  Existing: string;
  RootKey: integer;
  SubKey: string;
begin
  if IsAdminInstallMode then
  begin
    RootKey := HKEY_LOCAL_MACHINE;
    SubKey := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';
  end
  else
  begin
    RootKey := HKEY_CURRENT_USER;
    SubKey := 'Environment';
  end;
  if not RegQueryStringValue(RootKey, SubKey, 'Path', Existing) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Existing) + ';') = 0;
end;
