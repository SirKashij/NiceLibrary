# Does to the app's windows what the user and Windows do to them, for the
# end-to-end tests (e2e/windows.ts), through Win32 window messages. Windows
# PowerShell 5.1, which every Windows has.
#
#   -Action app      -ProcessId <tauri-driver>   the app process tauri-driver started
#   -Action windows  -ProcessId <app>            the app's top-level windows, as JSON
#   -Action post     -Window <hwnd> -Message <n> [-WParam <n>] [-LParam <n>]
#   -Action menu     -ProcessId <app> -Item <text>  chooses <text> in the app's open popup menu
param(
  [Parameter(Mandatory)] [ValidateSet('app', 'windows', 'post', 'menu')] [string] $Action,
  [int] $ProcessId,
  [long] $Window,
  [int] $Message,
  [long] $WParam = 0,
  [long] $LParam = 0,
  [string] $Item
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class E2eWindows {
  delegate bool EnumProc(IntPtr hwnd, IntPtr data);

  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr data);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassNameW(IntPtr hwnd, StringBuilder name, int size);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowTextW(IntPtr hwnd, StringBuilder text, int size);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);
  [DllImport("user32.dll")] static extern IntPtr SendMessageW(IntPtr hwnd, uint message, IntPtr wparam, IntPtr lparam);
  [DllImport("user32.dll")] static extern int GetMenuItemCount(IntPtr menu);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetMenuStringW(IntPtr menu, uint item, StringBuilder text, int size, uint flags);

  const uint MN_GETHMENU = 0x01E1;
  const uint MF_BYPOSITION = 0x0400;

  /** The entries of the popup menu shown in the window `hwnd` (class #32768), top to bottom. */
  public static List<string> MenuEntries(IntPtr hwnd) {
    var entries = new List<string>();
    var menu = SendMessageW(hwnd, MN_GETHMENU, IntPtr.Zero, IntPtr.Zero);
    if (menu == IntPtr.Zero) return entries;
    int count = GetMenuItemCount(menu);
    for (uint position = 0; position < count; position++) {
      var text = new StringBuilder(256);
      GetMenuStringW(menu, position, text, text.Capacity, MF_BYPOSITION);
      entries.Add(text.ToString());
    }
    return entries;
  }

  public class Window {
    public long hwnd;
    public string className;
    public string title;
    public bool visible;
  }

  public static List<Window> Of(uint pid) {
    var found = new List<Window>();
    EnumWindows((hwnd, data) => {
      uint owner;
      GetWindowThreadProcessId(hwnd, out owner);
      if (owner == pid) {
        var name = new StringBuilder(256);
        GetClassNameW(hwnd, name, name.Capacity);
        var text = new StringBuilder(256);
        GetWindowTextW(hwnd, text, text.Capacity);
        found.Add(new Window { hwnd = hwnd.ToInt64(), className = name.ToString(), title = text.ToString(), visible = IsWindowVisible(hwnd) });
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
'@

switch ($Action) {
  'app' {
    # The descendant of tauri-driver (through msedgedriver) that is the app.
    $processes = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
    $tree = @($ProcessId)
    do {
      $grown = @($processes | Where-Object { $tree -contains $_.ParentProcessId -and $tree -notcontains $_.ProcessId })
      $tree += $grown.ProcessId
    } while ($grown.Count -gt 0)
    $app = @($processes | Where-Object { $tree -contains $_.ProcessId -and $_.Name -like 'nicelibrary*' })
    if ($app.Count -ne 1) { throw "Expected one app process under tauri-driver $ProcessId, found $($app.Count)." }
    $app[0].ProcessId
  }
  'windows' {
    ConvertTo-Json -Compress -InputObject @([E2eWindows]::Of($ProcessId))
  }
  'post' {
    if (-not [E2eWindows]::PostMessageW([IntPtr]$Window, $Message, [IntPtr]$WParam, [IntPtr]$LParam)) {
      throw "PostMessage $Message to window $Window failed."
    }
  }
  'menu' {
    $deadline = (Get-Date).AddSeconds(10)
    do {
      # A popup menu is a top-level window of the class #32768.
      $menu = [E2eWindows]::Of($ProcessId) | Where-Object { $_.className -eq '#32768' -and $_.visible } | Select-Object -First 1
      if (-not $menu) { Start-Sleep -Milliseconds 200 }
    } while (-not $menu -and (Get-Date) -lt $deadline)
    if (-not $menu) { throw 'The app opened no popup menu within 10 s.' }
    $entries = @([E2eWindows]::MenuEntries([IntPtr]$menu.hwnd))
    $position = [array]::IndexOf($entries, $Item)
    if ($position -lt 0) { throw "The popup menu has no entry '$Item', only: '$($entries -join "', '")'." }
    # Chosen with the keyboard, as a user can: down to the entry, then Enter.
    $WM_KEYDOWN = 0x0100; $VK_DOWN = 0x28; $VK_RETURN = 0x0D
    for ($i = 0; $i -le $position; $i++) { [void][E2eWindows]::PostMessageW([IntPtr]$menu.hwnd, $WM_KEYDOWN, [IntPtr]$VK_DOWN, [IntPtr]0) }
    [void][E2eWindows]::PostMessageW([IntPtr]$menu.hwnd, $WM_KEYDOWN, [IntPtr]$VK_RETURN, [IntPtr]0)
  }
}
