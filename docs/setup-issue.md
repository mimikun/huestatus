# Hue ブリッジセットアップ時の問題と解決策

## 概要

Huestatus の初期セットアップ時に発生する可能性がある問題と、その診断・解決方法についてまとめています。

## 主な問題：ブリッジのボタンが点滅しない

### 症状

- セットアップ時に「ブリッジのボタンを押してください」と表示される
- ブリッジ上部のボタンを押しても青いLEDが点滅しない
- 認証が成功しない

### 正常な動作（BSB002モデルの場合）

Philips Hue Bridge 2015（BSB002）では、正常な場合以下の動作をします：

1. **ボタンを押すと**: 青いLEDが約10-15秒間点滅
2. **認証成功時**: 緑色のLEDが点灯
3. **認証失敗時**: 赤色のLEDが点灯

### 基本情報の確認

まず、あなたのブリッジの情報を確認してください：

```bash
curl http://192.168.1.146/api/0/config | jq '{modelid, bridgeid, swversion, apiversion}'
```

期待される出力例：

```json
{
  "modelid": "BSB002",
  "bridgeid": "ECB5FAFFFEBCA972",
  "swversion": "1972004020",
  "apiversion": "1.72.0"
}
```

**注意**: `bridgeid`の末尾（例：BCA972）は型番ではありません。型番は`modelid`で確認してください。

### 診断手順

#### 1. ネットワーク接続の確認

```bash
# ping テスト
ping -c 3 192.168.1.146

# HTTP接続テスト
curl -v http://192.168.1.146/api/0/config
```

正常な場合、ブリッジの設定情報が JSON 形式で返されます。

#### 2. 認証リクエストのテスト

```bash
# 認証なしリクエスト（ボタンを押す前）
curl -X POST -H "Content-Type: application/json" \
     -d '{"devicetype":"huestatus#test"}' \
     http://192.168.1.146/api
```

期待される応答：

```json
[
  {
    "error": {
      "type": 101,
      "address": "",
      "description": "link button not pressed"
    }
  }
]
```

#### 3. リアルタイム認証テスト

以下のコマンドを実行して、ボタンの動作を確認してください：

```bash
echo "=== テスト開始 ==="
echo "ベースライン確認:"
curl -X POST -d '{"devicetype":"test"}' http://192.168.1.146/api
echo ""

echo "=== 今すぐブリッジのボタンを1-2秒押してください ==="
echo "（ボタンを押したら5秒以内にEnterを押してください）"
read -r

echo "ボタンを押した直後のテスト:"
curl -X POST -d '{"devicetype":"test"}' http://192.168.1.146/api
echo ""

echo "10秒後のテスト:"
sleep 10
curl -X POST -d '{"devicetype":"test"}' http://192.168.1.146/api
```

## 考えられる原因と解決策

### 1. ボタンの位置・押し方の問題

**確認点:**

- BSB002では上面の大きな丸いボタンのみが有効
- 側面や下面のボタンは無関係
- 1-2秒の短押し（長押しではない）

**解決策:**

- ブリッジの真上から、中央の大きなボタンを確実に押す
- 「カチッ」という音が聞こえるまで押す

### 2. ブリッジの状態異常

**確認方法:**

```bash
# ブリッジの詳細情報を確認
curl -s http://192.168.1.146/description.xml | head -20
```

**解決策:**

1. **ブリッジの再起動**: 電源を10秒間切断してから再接続
2. **ネットワーク設定の確認**: ルーターとの接続状態を確認
3. **他のアプリケーションとの競合**: スマートフォンのHueアプリなどで接続テスト

### 3. WSL環境での問題

**症状確認:**

- Windows Subsystem for Linux (WSL) 使用時の特有の問題

**解決策:**

```bash
# WSL のネットワーク設定確認
cat /etc/resolv.conf
ip route show
```

WSL でミラーモードを使用している場合でも、稀にネットワーク問題が発生することがあります。
Windows側で直接テストして比較してください。

### 4. 複数デバイスでの認証試行

**問題:**
他のデバイス（スマートフォンアプリなど）で同時に認証を試行している

**解決策:**

1. 他のHueアプリをすべて閉じる
2. しばらく待ってから再試行
3. ブリッジの再起動

## 高度な診断

### ブリッジのログ確認

```bash
# ブリッジの現在の設定を詳細確認
curl -s http://192.168.1.146/api/0/config | jq '.'
```

### 認証済みユーザーの確認

もし既存の認証情報がある場合：

```bash
# 既存認証のテスト（USERNAME は実際の値に置換）
curl http://192.168.1.146/api/USERNAME/config
```

## よくある質問

### Q: ボタンが全く反応しない（LEDが一切光らない）

**A:** ハードウェアの問題の可能性があります：

1. 電源の確認（アダプターとケーブル）
2. 別のHueアプリでの動作確認
3. フィリップスサポートへの連絡

### Q: WSL環境では動作しないのか？

**A:** WSL環境でも正常に動作するはずです。ミラーモードが有効な場合、ネットワーク接続に問題はありません。

### Q: 認証成功後も接続できない

**A:** 設定ファイルの保存に問題がある可能性があります：

```bash
# 設定ファイルの場所確認
ls -la ~/.config/huestatus/
```

## 成功例

正常なセットアップの場合、以下のような流れになります：

```bash
$ cargo run -- setup --verbose
🏗️  Huestatus Setup
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Welcome to huestatus! Let's configure your Philips Hue lights.

⚙️ Step 1/7: Initializing setup...

🔍 Step 2/7: Discovering Hue bridges...
📡 Found 1 bridge(s) via Philips service

🔑 Step 3/7: Authenticating with bridge at 192.168.1.146...
🔑 Authentication Required

To authenticate with your Hue bridge:
1. Press the large button on top of your Hue bridge
2. Wait for the button to start blinking  # <- ここで青いLEDが点滅するはず
3. Press Enter to continue

✅ Authentication successful! Username: abc123def456...
```

## 緊急時の手動設定

もしすべての自動設定が失敗する場合、手動で設定ファイルを作成できます：

```bash
mkdir -p ~/.config/huestatus
cat > ~/.config/huestatus/config.json << 'EOF'
{
  "bridge": {
    "ip": "192.168.1.146",
    "username": "手動で取得したユーザー名"
  }
}
EOF
```

手動でのユーザー名取得：

```bash
# 1. ボタンを押す
# 2. 30秒以内に実行
curl -X POST -d '{"devicetype":"huestatus#manual"}' http://192.168.1.146/api
```

成功時の応答例：

```json
[{ "success": { "username": "abc123def456..." } }]
```

---

**問題が解決しない場合は、GitHub Issues で詳細な環境情報と併せて報告してください。**

