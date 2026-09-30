# Read-only app integration test fixture: publishes synthetic media metadata and
# authoritative timeline updates, without audio or access to any player account.
# This is a test session, never evidence that Spotify/NetEase synchronization works.
param([int]$Seconds = 90)
$ErrorActionPreference = 'Stop'
$null = [Windows.Media.Playback.MediaPlayer, Windows.Media.Playback, ContentType = WindowsRuntime]
$null = [Windows.Media.SystemMediaTransportControlsTimelineProperties, Windows.Media, ContentType = WindowsRuntime]
$null = [Windows.Media.MediaPlaybackStatus, Windows.Media, ContentType = WindowsRuntime]
$null = [Windows.Media.MediaPlaybackType, Windows.Media, ContentType = WindowsRuntime]
$fixturePlayer = New-Object Windows.Media.Playback.MediaPlayer
try {
    $fixturePlayer.CommandManager.IsEnabled = $false
    $fixtureSmtc = $fixturePlayer.SystemMediaTransportControls
    $fixtureSmtc.IsEnabled = $true
    $fixtureSmtc.IsPlayEnabled = $true
    $fixtureSmtc.IsPauseEnabled = $true
    $fixtureUpdater = $fixtureSmtc.DisplayUpdater
    $fixtureUpdater.Type = [Windows.Media.MediaPlaybackType]::Music
    $fixtureUpdater.MusicProperties.Title = 'I Want to Live'
    $fixtureUpdater.MusicProperties.Artist = 'Borislav Slavov'
    $fixtureUpdater.MusicProperties.AlbumTitle = "Baldur's Gate 3 (Original Game Soundtrack)"
    $fixtureUpdater.Update()
    $fixtureTimeline = New-Object Windows.Media.SystemMediaTransportControlsTimelineProperties
    $fixtureTimeline.StartTime = [TimeSpan]::Zero
    $fixtureTimeline.EndTime = [TimeSpan]::FromSeconds(233)
    $fixtureTimeline.MinSeekTime = [TimeSpan]::Zero
    $fixtureTimeline.MaxSeekTime = [TimeSpan]::FromSeconds(233)
    $fixturePosition = 25
    Write-Output 'Synthetic SMTC fixture: public LRCLIB sample metadata; no audio.'
    for ($fixtureTick = 0; $fixtureTick -lt $Seconds; $fixtureTick++) {
        $fixturePaused = $fixtureTick -ge 20 -and $fixtureTick -lt 25
        $fixtureSmtc.PlaybackStatus = if ($fixturePaused) { [Windows.Media.MediaPlaybackStatus]::Paused } else { [Windows.Media.MediaPlaybackStatus]::Playing }
        if ($fixtureTick -eq 40) { $fixturePosition = 80; Write-Output 'Fixture seek to 80s' }
        if (-not $fixturePaused) { $fixturePosition++ }
        $fixtureTimeline.Position = [TimeSpan]::FromSeconds($fixturePosition)
        $fixtureSmtc.UpdateTimelineProperties($fixtureTimeline)
        Start-Sleep -Seconds 1
    }
} finally {
    $fixtureSmtc.IsEnabled = $false
    $fixturePlayer.Dispose()
}
