$b=[IO.File]::ReadAllBytes("C:\Users\mouss\Music\imgui2\target\debug\imruski_payload.dll")
$m=[IO.MemoryStream]::new($b); $r=[IO.BinaryReader]::new($m)
$m.Position=0x3C; $lf=$r.ReadUInt32()
$m.Position=$lf+4+20  # skip sig + file header
$optMagic=$r.ReadUInt16()
Write-Host "OptMagic=0x$($optMagic.ToString('X'))"
$m.Position=$lf+4
$numSecs=$r.ReadBytes(20) | Out-Null
$m.Position=$lf+4; $fh=[IO.BinaryReader]::new([IO.MemoryStream]::new($b))
$fh.BaseStream.Position=$lf+6; $ns=[BitConverter]::ToUInt16($b,$lf+6)
Write-Host "NumSections=$ns"
# Size of optional header
$soh=[BitConverter]::ToUInt16($b,$lf+20)
Write-Host "SizeOfOptHdr=$soh"
$secStart=$lf+24+$soh
Write-Host "SectionHeadersAt=0x$($secStart.ToString('X'))"
for($i=0;$i -lt $ns;$i++){
    $so=$secStart+$i*40
    $name=[System.Text.Encoding]::ASCII.GetString($b,$so,8).TrimEnd([char]0)
    $vsz=[BitConverter]::ToUInt32($b,$so+8)
    $rva=[BitConverter]::ToUInt32($b,$so+12)
    Write-Host "Section[$i]: name='$name' va=0x$($rva.ToString('X')) vsz=$vsz"
}
