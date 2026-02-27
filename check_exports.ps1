$b=[IO.File]::ReadAllBytes("C:\Users\mouss\Music\imgui2\target\debug\imruski_payload.dll")
$m=[IO.MemoryStream]::new($b); $r=[IO.BinaryReader]::new($m)
$m.Position=0x3C; $lf=$r.ReadUInt32()
Write-Host "lfanew=$($lf.ToString('X'))"
$m.Position=$lf+24+112; $erva=$r.ReadUInt32(); $esz=$r.ReadUInt32()
Write-Host "ExportDir: rva=$($erva.ToString('X')) size=$esz"
$m.Position=$erva; $raw=$r.ReadBytes(40)
Write-Host "Raw: $([BitConverter]::ToString($raw))"
$nf=[BitConverter]::ToUInt32($raw,20); $nn=[BitConverter]::ToUInt32($raw,24)
$af=[BitConverter]::ToUInt32($raw,28); $an=[BitConverter]::ToUInt32($raw,32)
Write-Host "nf=$nf nn=$nn af=$($af.ToString('X')) an=$($an.ToString('X'))"
if($nn -gt 0 -and $an -lt $b.Length){
    for($i=0;$i -lt $nn;$i++){
        $m.Position=$an+$i*4; $nr=$r.ReadUInt32()
        if($nr -gt 0 -and $nr -lt $b.Length){ $m.Position=$nr; $s=""; for($j=0;$j -lt 100;$j++){$c=$r.ReadByte();if($c -eq 0){break};$s+=[char]$c}; Write-Host "Name[$i]: $s" }
    }
}
$r.Close()
