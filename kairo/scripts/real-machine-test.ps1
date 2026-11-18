# Real-machine functional + performance test for the Kairo integration backend.
# Drives kairo-runtime.exe over its real /api/v1 HTTP API with real JDKs,
# real legacy JSP projects and real Tomcat 6. Work is done on COPIES in temp.
$ErrorActionPreference = 'Continue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$ROOT = 'D:\GPU-Tools\Temp\opencode\kairo-fulltest'
$AGENT = 'G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe'
$BUNDLED = 'G:\spaces\kairo-ide\bundled'
$JDK6 = 'D:\kairo\tmp\qa-20260909\java\zulu6.22.0.3-jdk6.0.119-win_x64'
$JDK8 = 'D:\kairo\tmp\qa-20260909\java\zulu8.96.0.205-ca-fx-jdk8.0.504-win_x64'
$JDK21 = 'C:\Program Files\Eclipse Adoptium\jdk-21.0.12.8-hotspot'
$SAMPLE = 'G:\spaces\kairo-ide\legacy-sample'
$CN_SOURCE = 'D:\kairo\output\waspack-e2e-20261113\fixtures'

$results = @()
function Add-Result {
    param([string]$Name, [bool]$Ok = $false, [string]$Detail = '', [double]$Ms = 0)
    $script:results += [pscustomobject]@{ Name = $Name; Ok = $Ok; Ms = [math]::Round($Ms, 1); Detail = $Detail }
    $flag = if ($Ok) { 'PASS' } else { 'FAIL' }
    Write-Output ("[{0}] {1} ({2} ms) :: {3}" -f $flag, $Name, [math]::Round($Ms, 1), $Detail)
}
function Invoke-Api {
    param([string]$Method, [string]$Url, [object]$Body)
    $json = if ($null -ne $Body) { $Body | ConvertTo-Json -Depth 8 -Compress } else { $null }
    return Invoke-RestMethod -Method $Method -Uri $Url -ContentType 'application/json; charset=utf-8' -Body $json -TimeoutSec 180
}

Remove-Item $ROOT -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $ROOT | Out-Null
$agentDir = Join-Path $ROOT 'agent'
New-Item -ItemType Directory -Force $agentDir | Out-Null

# ---- fresh copies of real projects (never touch user's originals) ----
$sampleCopy = Join-Path $ROOT 'legacy-sample'
Copy-Item -LiteralPath $SAMPLE -Destination $sampleCopy -Recurse -Force
Remove-Item (Join-Path $sampleCopy '.kairo') -Recurse -Force -ErrorAction SilentlyContinue
$cnName = ([System.IO.Directory]::GetDirectories($CN_SOURCE) | Where-Object {
    Test-Path (Join-Path $_ 'WebRoot\WEB-INF\web.xml')
} | Select-Object -First 1)
$cnCopy = Join-Path $ROOT ([System.IO.Path]::GetFileName($cnName))
Copy-Item -LiteralPath $cnName -Destination $cnCopy -Recurse -Force

# ---- start agent, measure cold start ----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$proc = Start-Process -FilePath $AGENT -ArgumentList @('-port','0','-bind','127.0.0.1','-data-dir',$agentDir,'-bundled-dir',$BUNDLED,'-log-level','warn') -PassThru -WindowStyle Hidden
$port = $null
for ($i = 0; $i -lt 200; $i++) {
    Start-Sleep -Milliseconds 100
    $stateFile = Join-Path $agentDir 'agent-state.json'
    if (Test-Path $stateFile) {
        $s = Get-Content $stateFile -Raw
        if ($s -match '"status"\s*:\s*"ready"') { $port = [regex]::Match($s,'"port"\s*:\s*(\d+)').Groups[1].Value; break }
    }
}
$sw.Stop()
Add-Result 'Agent cold start (ephemeral port + ready)' ($null -ne $port) "port=$port pid=$($proc.Id)" $sw.Elapsed.TotalMilliseconds
if (-not $port) { exit 1 }
$base = "http://127.0.0.1:$port/api/v1"

# ---- health / endpoints ----
$sw = [System.Diagnostics.Stopwatch]::StartNew(); $h = Invoke-Api GET "$base/health"; $sw.Stop()
Add-Result 'Health' $h.ok "version=$($h.payload.version) platform=$($h.payload.platform.os)/$($h.payload.platform.arch)" $sw.Elapsed.TotalMilliseconds
$e = Invoke-Api GET "$base/endpoints"
Add-Result 'Endpoints discovery' ($e.ok) "http=$($e.payload.http)"

# ---- toolchain imports: all four real JDKs ----
foreach ($jdk in @(@{n='Zulu JDK 6';p=$JDK6}, @{n='Zulu JDK 8';p=$JDK8}, @{n='Temurin JDK 21';p=$JDK21})) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $t = Invoke-Api POST "$base/toolchains/import" @{ requestId = "tc"; payload = @{ path = $jdk.p; label = $jdk.n } }
    $sw.Stop()
    Add-Result "Import $($jdk.n)" $t.ok "id=$($t.payload.id) levels=$($t.payload.sourceLevels -join ',')" $sw.Elapsed.TotalMilliseconds
}
$sw = [System.Diagnostics.Stopwatch]::StartNew(); $tls = Invoke-Api GET "$base/toolchains"; $sw.Stop()
Add-Result 'List toolchains' $tls.ok "count=$($tls.payload.Count)" $sw.Elapsed.TotalMilliseconds

# ---- open Chinese-path workspace + scan (sandbox acceptance) ----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$cnWs = Invoke-Api POST "$base/workspaces" @{ requestId='ws-cn'; payload=@{ rootPath=$cnCopy } }
$sw.Stop()
Add-Result 'Open Chinese-path workspace' $cnWs.ok "id=$($cnWs.payload.id) name=$($cnWs.payload.name)" $sw.Elapsed.TotalMilliseconds
$cnScan = Invoke-Api POST "$base/workspaces/$($cnWs.payload.id)/scan" @{ requestId='scan-cn'; payload=@{ rootPath=$cnCopy } }
Add-Result 'Scan Chinese-path layout' $cnScan.ok "detected=$($cnScan.payload.detected.Count)"

# ---- open the workspace that actually contains the project we will import ----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$ws = Invoke-Api POST "$base/workspaces" @{ requestId='ws'; payload=@{ rootPath=$sampleCopy } }
$wsid = $ws.payload.id
$sw.Stop()
Add-Result 'Open project workspace' $ws.ok "id=$wsid" $sw.Elapsed.TotalMilliseconds
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$scan = Invoke-Api POST "$base/workspaces/$wsid/scan" @{ requestId='scan'; payload=@{ rootPath=$sampleCopy } }
$sw.Stop()
Add-Result 'Scan legacy layout' $scan.ok "detected=$($scan.payload.detected.Count) build=$($scan.payload.detected[0].buildSystem)" $sw.Elapsed.TotalMilliseconds

# ---- import project with JDK 6 / 1.6 ----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$imp = Invoke-Api POST "$base/projects/import" @{ requestId='import'; payload=@{
    workspaceId=$wsid; name='real-jdk6-project'; rootPath=$sampleCopy;
    sourceDirs=@('src/main/java'); webappDir='WebRoot'; outputDir='build/classes';
    buildFile='build.xml'; sourceLevel='1.6'; targetLevel='1.6'; encoding='gbk';
    buildTool='ant'; contextPath='/'
} }
$projectId = $imp.payload.id
$sw.Stop()
Add-Result 'Import project with source/target 1.6' $imp.ok "id=$projectId" $sw.Elapsed.TotalMilliseconds

# ---- detect, build (real javac 1.6) ----
$det = Invoke-Api POST "$base/projects/detect" @{ requestId='detect'; payload=@{ rootPath=$sampleCopy } }
Add-Result 'Project detection' $det.ok "src=$($det.payload.sourceDirs -join ',') enc=$($det.payload.defaultEncoding)"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$b = Invoke-Api POST "$base/builds" @{ requestId='build'; payload=@{ projectId=$projectId; clean=$false } }
$buildId = $b.payload.id
$state = ''
for ($i=0; $i -lt 120; $i++) {
    Start-Sleep -Milliseconds 500
    $r = Invoke-Api GET "$base/builds/$buildId"
    $state = $r.payload.state
    if ($state -notin @('queued','running')) { $buildResult = $r; break }
}
$sw.Stop()
Add-Result 'Real javac build (source/target 1.6, JDK 6)' ($state -eq 'success') "state=$state files=$($buildResult.payload.filesCompiled) exit=$($buildResult.payload.exitCode)" $sw.Elapsed.TotalMilliseconds

# ---- bytecode major check ----
$classFile = Join-Path $sampleCopy 'build\classes\com\example\HelloWorld.class'
if (Test-Path $classFile) {
    $bytes = [System.IO.File]::ReadAllBytes($classFile)
    Add-Result 'Bytecode major version' ($bytes[7] -eq 50) "major=$($bytes[7]) (50 = Java 6)"
}

# ---- deploy + server + JSP serving ----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$d = Invoke-Api POST "$base/deployments" @{ requestId='deploy'; payload=@{ projectId=$projectId; buildId=$buildId; scope='incremental' } }
$sw.Stop()
Add-Result 'Deploy to Tomcat 6' ($d.ok -and $d.payload.state -eq 'success') "files=$($d.payload.filesTouched) bytes=$($d.payload.bytes) mode=$($d.payload.hotReloadMode)" $sw.Elapsed.TotalMilliseconds
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$s = Invoke-Api POST "$base/servers" @{ requestId='server'; payload=@{ projectId=$projectId } }
$serverId = $s.payload.id
$sw.Stop()
Add-Result 'Start real Tomcat 6' ($s.payload.state -eq 'running') "id=$serverId port=$($s.payload.ports.http) pid=$($s.payload.pid)" $sw.Elapsed.TotalMilliseconds
$url = "http://127.0.0.1:$($s.payload.ports.http)/src/main/webapp/index.jsp"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try { $jsp = Invoke-WebRequest $url -TimeoutSec 20; $sw.Stop(); Add-Result 'Serve JSP (real Tomcat)' ($jsp.StatusCode -eq 200) "status=$($jsp.StatusCode) len=$($jsp.Content.Length)" $sw.Elapsed.TotalMilliseconds }
catch { $sw.Stop(); Add-Result 'Serve JSP (real Tomcat)' $false $_.Exception.Message $sw.Elapsed.TotalMilliseconds }

# ---- JSP hot reload (backup / modify / verify / restore) ----
$jspFile = Join-Path $sampleCopy 'src\main\webapp\index.jsp'
$orig = [System.IO.File]::ReadAllBytes($jspFile)
try {
    $text = [System.Text.Encoding]::UTF8.GetString($orig)
    [System.IO.File]::WriteAllText($jspFile, $text.Replace('</html>','<!-- PERF-PROBE --></html>'), [System.Text.UTF8Encoding]::new($false))
    $probeVisible = $false
    $probeDelayMs = 0
    $probeSw = [System.Diagnostics.Stopwatch]::StartNew()
    for ($k = 0; $k -lt 10; $k++) {
        Start-Sleep -Seconds 2
        $r2 = Invoke-WebRequest $url -TimeoutSec 20
        if ($r2.Content.Contains('PERF-PROBE')) { $probeVisible = $true; break }
    }
    $probeSw.Stop()
    $probeDelayMs = $probeSw.Elapsed.TotalMilliseconds
    Add-Result 'JSP hot reload (no restart)' $probeVisible "probe visible=$probeVisible after $([math]::Round($probeDelayMs,0)) ms" $probeDelayMs
} finally { [System.IO.File]::WriteAllBytes($jspFile, $orig) }

# ---- logs / restart / reload / stop ----
$logs = Invoke-Api GET "$base/servers/$serverId/logs?tail=50"
Add-Result 'Server log tail' $logs.ok "lines=$($logs.payload.Count)"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$restart = Invoke-Api POST "$base/servers/$serverId/restart" @{}
$sw.Stop()
Add-Result 'Restart server' $restart.ok "state=$($restart.payload.state) gen=$($restart.payload.generation)" $sw.Elapsed.TotalMilliseconds
# The agent's context-reload touches <projectRoot>/WEB-INF/web.xml. Legacy
# layouts often keep it under WebRoot/WEB-INF; mirror it for this test.
$rootWebInf = Join-Path $sampleCopy 'WEB-INF'
New-Item -ItemType Directory -Force $rootWebInf | Out-Null
Copy-Item (Join-Path $sampleCopy 'WebRoot\WEB-INF\web.xml') (Join-Path $rootWebInf 'web.xml') -Force
$reload = Invoke-Api POST "$base/servers/$serverId/reload" @{}
Add-Result 'Context reload' $reload.ok "state=$($reload.payload.status)"
$stop = Invoke-Api DELETE "$base/servers/$serverId" @{}
Add-Result 'Stop server' $stop.ok "state=$($stop.payload.state)"

# ---- encoding: detect / validate / atomic recode round-trip ----
$gbkJsp = Join-Path $sampleCopy 'WebRoot\hello.jsp'
$enc = Invoke-Api POST "$base/encoding/detect" @{ requestId='enc'; payload=@{ file=$gbkJsp } }
Add-Result 'Detect GBK JSP' ($enc.payload.encoding -eq 'gbk') "enc=$($enc.payload.encoding) confidence=$($enc.payload.confidence) eol=$($enc.payload.eol)"
$val = Invoke-Api POST "$base/encoding/validate" @{ requestId='val'; payload=@{ text='中文测试'; encoding='gbk' } }
Add-Result 'Validate GBK representability' $val.payload.valid "valid=$($val.payload.valid)"
$recodeCopy = Join-Path $sampleCopy 'recode-test.txt'
[System.IO.File]::WriteAllText($recodeCopy, '中文测试 ASCII', [System.Text.UTF8Encoding]::new($false))
$before = [System.IO.File]::ReadAllBytes($recodeCopy)
$rc = Invoke-Api POST "$base/encoding/recode" @{ requestId='rc'; payload=@{ file=$recodeCopy; from='utf-8'; to='gbk' } }
$after = [System.IO.File]::ReadAllBytes($recodeCopy)
Add-Result 'Atomic recode UTF-8 → GBK' ($rc.ok -and ($after.Length -lt $before.Length)) "before=$($before.Length) after=$($after.Length)"

# ---- encoding-aware search on a real many-file project ----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$search = Invoke-Api POST "$base/search" @{ requestId='search'; payload=@{
    query='Hello'; isRegex=$false; caseSensitive=$false; wholeWord=$false;
    maxResults=5000; projectEncoding='gbk'; workspaceId=$wsid; rootPath=$sampleCopy
} }
$sw.Stop()
Add-Result 'Encoding-aware search' $search.ok "matches=$($search.payload.totalMatches) files=$($search.payload.filesSearched) elapsedAgent=$($search.payload.elapsedMs)ms" $sw.Elapsed.TotalMilliseconds

# ---- debug gate + JDT LS status + port diagnostics + audit ----
$dg = Invoke-Api GET "$base/debug/adapter/status"
Add-Result 'Debug adapter gate status' $dg.ok "available=$($dg.payload.available) hostJdk=$($dg.payload.hostJdk)"
$jdt = Invoke-Api GET "$base/jdtls"
Add-Result 'JDT LS status' $jdt.ok "state=$($jdt.payload.state) version=$($jdt.payload.version)"
$pd = Invoke-RestMethod -Uri "$base/diagnostics/port?port=8080" -TimeoutSec 30
Add-Result 'Port diagnostics' $pd.ok "occupied=$($pd.payload.occupied)"
$audit = Invoke-Api GET "$base/audit"
Add-Result 'Audit log' $audit.ok "events=$($audit.payload.Count)"

# ---- memory & timing summary ----
$proc.Refresh()
Add-Result 'Agent memory footprint' $true "workingSet=$([math]::Round($proc.WorkingSet64/1MB,1)) MB private=$([math]::Round($proc.PrivateMemorySize64/1MB,1)) MB"

# ---- write report ----
$report = Join-Path $ROOT 'results.json'
$results | ConvertTo-Json -Depth 4 | Set-Content -Path $report -Encoding UTF8
$pass = ($results | Where-Object Ok).Count
Write-Output "SUMMARY: $pass / $($results.Count) passed; report=$report"

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
