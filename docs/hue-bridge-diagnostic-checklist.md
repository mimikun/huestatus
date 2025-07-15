# Hue ブリッジ診断チェックリスト

以下の項目を順番に実行して、問題を特定してください。

## 基本情報確認

- [ ] ブリッジのモデル確認
  ```bash
  curl -s http://192.168.1.146/api/0/config | jq '{modelid, bridgeid, swversion}'
  ```
  - 期待値: `modelid: "BSB002"`

## ネットワーク診断

- [x] **完了**: ping テスト
  ```bash
  ping -c 3 192.168.1.146
  ```

- [x] **完了**: HTTP接続テスト
  ```bash
  curl -v http://192.168.1.146/api/0/config
  ```

## 認証API診断

- [x] **完了**: 認証リクエストテスト（ボタンを押す前）
  ```bash
  curl -X POST -H "Content-Type: application/json" \
       -d '{"devicetype":"huestatus#test"}' \
       http://192.168.1.146/api
  ```
  - 期待値: `[{"error":{"type":101,"address":"","description":"link button not pressed"}}]`
  - 結果: ✅ 期待通りのエラー101が返された

- [x] **完了**: リアルタイム認証テスト（ボタン動作確認）
  ```bash
  echo "=== ボタンを押す前のテスト ==="
  curl -X POST -d '{"devicetype":"test"}' http://192.168.1.146/api
  echo ""
  echo "=== 今すぐブリッジのボタンを1-2秒押してください ==="
  echo "（ボタンを押したらEnterを押してください）"
  read -r
  echo "=== ボタンを押した直後のテスト ==="
  curl -X POST -d '{"devicetype":"test"}' http://192.168.1.146/api
  echo ""
  echo "=== 10秒後のテスト ==="
  sleep 10
  curl -X POST -d '{"devicetype":"test"}' http://192.168.1.146/api
  ```
  
  **テスト結果（2025-07-15）**:
  ```
  === ボタンを押す前のテスト ===
  [{"error":{"type":101,"address":"","description":"link button not pressed"}}]
  === 今すぐブリッジのボタンを1-2秒押してください ===
  （ボタンを押したらEnterを押してください）
  
  === ボタンを押した直後のテスト ===
  [{"success":{"username":"qf5lBV09NdJM-g1V6Ozczj4NDiWxTnjB6v86-8y5"}}]
  === 10秒後のテスト ===
  [{"error":{"type":101,"address":"","description":"link button not pressed"}}]
  ```
  
  **重要な発見**:
  - ✅ 認証処理は正常に動作
  - ⚠️ 認証ウィンドウは約10秒以内と非常に短い
  - ⚠️ タイミングがシビアなためCLI実装時は注意が必要
  - 💡 実装時の考慮点: ユーザーへの明確な指示、適切なエラーハンドリング、タイムアウト対応

## 物理的な確認

- [ ] ブリッジの電源LED確認
  - 正常: 青色で点灯

- [ ] ボタンの位置確認
  - BSB002: 上面の大きな丸いボタン

- [ ] ボタンを押した時のLED動作確認
  - 期待される動作:
    - [ ] ボタンを押すと青いLEDが点滅開始
    - [ ] 約10-15秒間点滅継続
    - [ ] 認証成功で緑色、失敗で赤色

## トラブルシューティング

- [ ] ブリッジの再起動
  ```bash
  # 1. 電源ケーブルを抜く
  # 2. 10秒待つ
  # 3. 電源ケーブルを再接続
  # 4. 1-2分待って完全起動を確認
  curl http://192.168.1.146/api/0/config
  ```

- [ ] 他のHueアプリとの競合確認
  - [ ] スマートフォンのHueアプリを閉じる
  - [ ] 他のHue関連ソフトウェアを停止

- [ ] 別デバイスでの動作確認
  - [ ] スマートフォンのHueアプリで接続テスト
  - [ ] Windowsから直接テスト（WSL以外）

## 高度な診断

- [ ] ブリッジのUPnP情報確認
  ```bash
  curl -s http://192.168.1.146/description.xml | grep -E "(modelName|modelNumber|serialNumber)"
  ```

- [ ] 既存認証の確認（もしある場合）
  ```bash
  ls -la ~/.config/huestatus/
  cat ~/.config/huestatus/config.json
  ```

## WSL環境特有の確認

- [ ] WSLネットワーク設定確認
  ```bash
  cat /etc/resolv.conf
  ip route show
  ```

- [ ] WSLとWindows間での動作比較
  - [ ] WindowsのPowerShellで同じcurlコマンドを実行
  - [ ] 結果を比較

## 緊急時の手動設定

もしすべて失敗する場合：

- [ ] 手動認証の実行
  ```bash
  # 1. ボタンを押す
  # 2. 30秒以内に実行
  curl -X POST -d '{"devicetype":"huestatus#manual"}' http://192.168.1.146/api
  ```

- [ ] 設定ファイルの手動作成
  ```bash
  mkdir -p ~/.config/huestatus
  # 取得したusernameを使用して設定ファイル作成
  ```

## 問題報告用情報

問題が解決しない場合、以下の情報を収集：

- [ ] ブリッジ情報
  ```bash
  curl -s http://192.168.1.146/api/0/config | jq '{modelid, bridgeid, swversion, apiversion}'
  ```

- [ ] 環境情報
  ```bash
  uname -a
  cat /etc/os-release
  ```

- [ ] ネットワーク情報
  ```bash
  ip addr show
  ip route show
  ```

---

**現在の状況**: ネットワーク接続は正常、次は認証APIテストを実行中