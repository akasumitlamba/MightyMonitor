; Per-user app installer. Runtime is downloaded from its original vendor, not bundled.
#ifndef AppVersion
  #define AppVersion "0.0.1-alpha.7"
#endif
#define RuntimeVersion "1.28.8"
#define RuntimeInstaller "gstreamer-1.0-msvc-x86_64-" + RuntimeVersion + ".exe"
#define RuntimeSHA256 "e4c41ee6ee9c6c32a45d198463ec08d79bac3bd5fa58adc93d13fe4a08abc9d9"
[Setup]
AppId={{11586C82-2720-4F34-992A-E2E907946D3F}
AppName=MightyMonitor
AppVersion={#AppVersion}
AppPublisher=MightyMonitor contributors
AppPublisherURL=https://github.com/reduckyLang/MightyMonitor
DefaultDirName={localappdata}\Programs\MightyMonitor
DefaultGroupName=MightyMonitor
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\..\output\installers
OutputBaseFilename=MightyMonitor-{#AppVersion}-windows-x64-setup
SetupIconFile=mightymonitor.ico
UninstallDisplayIcon={app}\mightymonitor.ico
LicenseFile=..\..\LICENSE
InfoBeforeFile=installer-info.txt
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
[Files]
Source: "..\..\target\release\mm-viewer.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\mm-host.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\mm-launcher.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "mightymonitor.ico"; DestDir: "{app}"
Source: "..\..\LICENSE"; DestDir: "{app}"
Source: "..\..\NOTICE"; DestDir: "{app}"
[Registry]
Root: HKCU; Subkey: "Software\Classes\mightymonitor"; ValueType: string; ValueData: "URL:MightyMonitor connection"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\mightymonitor"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\mightymonitor\DefaultIcon"; ValueType: string; ValueData: """{app}\mightymonitor.ico"""
Root: HKCU; Subkey: "Software\Classes\mightymonitor\shell\open\command"; ValueType: string; ValueData: """{app}\mm-launcher.exe"" ""%1"""
[Icons]
Name: "{group}\MightyMonitor"; Filename: "{app}\mm-launcher.exe"; IconFilename: "{app}\mightymonitor.ico"
Name: "{autodesktop}\MightyMonitor"; Filename: "{app}\mm-launcher.exe"; IconFilename: "{app}\mightymonitor.ico"; Tasks: desktopicon
[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked
[Run]
Filename: "{app}\mm-launcher.exe"; Description: "Open MightyMonitor"; Flags: nowait postinstall skipifsilent
[Code]
function RuntimeRoot: String;
begin
  Result := ExpandConstant('{localappdata}\MightyMonitor\gstreamer-{#RuntimeVersion}');
end;
function RuntimePresent: Boolean;
begin
  Result := FileExists(RuntimeRoot + '\bin\gstreamer-1.0-0.dll') and
    FileExists(RuntimeRoot + '\lib\gstreamer-1.0\gstlibav.dll') and
    FileExists(RuntimeRoot + '\lib\gstreamer-1.0\gstx264.dll');
end;
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Result := '';
  if RuntimePresent then exit;
  try
    WizardForm.StatusLabel.Caption := 'Downloading the video runtime...';
    DownloadTemporaryFile('https://gstreamer.freedesktop.org/data/pkg/windows/{#RuntimeVersion}/msvc/{#RuntimeInstaller}', '{#RuntimeInstaller}', '{#RuntimeSHA256}', nil);
    WizardForm.StatusLabel.Caption := 'Installing the video runtime...';
    if not Exec(ExpandConstant('{tmp}\{#RuntimeInstaller}'), '/VERYSILENT /NORESTART /CURRENTUSER /TYPE=devel /DIR="' + RuntimeRoot + '"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
      RaiseException('Could not start the video runtime installer.');
    if ResultCode <> 0 then RaiseException('Video runtime installation failed: ' + IntToStr(ResultCode));
    if not RuntimePresent then RaiseException('Required video runtime components were not installed.');
  except
    Result := 'MightyMonitor could not install its video runtime. Check your internet connection and run Setup again.' + #13#10 + GetExceptionMessage;
  end;
end;
