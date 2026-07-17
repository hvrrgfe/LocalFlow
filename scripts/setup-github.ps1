# LocalFlow GitHub Setup Script
# Run this script in PowerShell to publish LocalFlow to GitHub

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  LocalFlow GitHub Setup" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
$repoName = "LocalFlow"
$repoDescription = "Local-first AI Agent & Workflow orchestration tool"

$ghPath = Get-Command gh -ErrorAction SilentlyContinue
if (-not $ghPath) {
    Write-Host "ERROR: GitHub CLI (gh) not found." -ForegroundColor Red
    Write-Host "Install from: https://cli.github.com/" -ForegroundColor Yellow
    exit 1
}
Write-Host "Step 1: Found gh CLI" -ForegroundColor Green

$authStatus = gh auth status 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Step 2: Login needed" -ForegroundColor Yellow
    Write-Host "Create a token at https://github.com/settings/tokens (scopes: repo, workflow)"
    $token = Read-Host "Paste token"
    if ([string]::IsNullOrWhiteSpace($token)) { exit 1 }
    $env:GH_TOKEN = $token
    $result = gh auth login --with-token 2>&1
    if ($LASTEXITCODE -ne 0) { Write-Host "Login failed"; exit 1 }
    Write-Host "Logged in!" -ForegroundColor Green
} else {
    Write-Host "Step 2: Already logged in" -ForegroundColor Green
}

Write-Host "Step 3: Creating repo..." -ForegroundColor Yellow
gh repo create "$repoName" --public --description "$repoDescription" 2>&1
if ($LASTEXITCODE -ne 0) { Write-Host "Repo may already exist" -ForegroundColor Yellow }

Write-Host "Step 4: Pushing code..." -ForegroundColor Yellow
git remote remove origin 2>$null
git remote add origin "https://github.com/$repoName.git" 2>&1
git push -u origin master 2>&1
if ($LASTEXITCODE -ne 0) { Write-Host "Push failed" -ForegroundColor Red; exit 1 }
Write-Host "Pushed!" -ForegroundColor Green

Write-Host "Step 5: Creating release..." -ForegroundColor Yellow
$exePath = "D:\Steam\LocalFlow\target\release\localflow-desktop.exe"
if (Test-Path $exePath) {
    $tagName = "v0.1.0"
    git tag -d $tagName 2>$null
    git tag $tagName 2>&1
    git push origin $tagName 2>&1
    gh release create $tagName --title "LocalFlow v0.1.0" --notes "MVP release" $exePath 2>&1
    Write-Host "Release created!" -ForegroundColor Green
} else {
    Write-Host "EXE not found at $exePath" -ForegroundColor Yellow
}

Write-Host "Done! https://github.com/$repoName" -ForegroundColor Cyan