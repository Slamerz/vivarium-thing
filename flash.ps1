$packageRoot = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory |
  Where-Object Name -like "AVRDudes.AVRDUDE*"

$realAvrdude = Get-ChildItem $packageRoot.FullName -Recurse -Filter avrdude.exe |
  Select-Object -First 1 -ExpandProperty FullName

$avrdudeConf = Get-ChildItem $packageRoot.FullName -Recurse -Filter avrdude.conf |
  Select-Object -First 1 -ExpandProperty FullName

& $realAvrdude `
  -C $avrdudeConf `
  -c arduino `
  -p atmega328p `
  -P COM4 `
  -b 115200 `
  -D `
  -U flash:w:.\vivarium-uno.hex:i