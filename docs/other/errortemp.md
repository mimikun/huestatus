いま一度セットアップを実行してきました.

まずHueブリッジの大きな丸いボタンを押し, すぐにターミナルで `huestatus setup` を実行しました.

すると以下の表示が出てきました.

ただし, `#` からはじまる行は注釈であり、実際には表示されていないことに注意してください.

```txt
Welcome to huestatus! Let's configure your Philips Hue lights.

⚙️ Step 1/7: Initializing setup...

🔍 Step 2/7: Discovering Hue bridges...

🔑 Step 3/7: Authenticating with bridge at 192.168.1.146...

🔑 Authentication Required

To authenticate with your Hue bridge:
1. Press the large button on top of your Hue bridge
2. Wait for the button to start blinking
3. Press Enter to continue
# Enterを押す前に、Hueブリッジの大きな丸いボタンを押す
```

すると認証ステップが次に進みますが, エラーが表示されます.

```txt
Press the bridge button now and then press Enter...🔍 Attempting authentication...
⏳ Waiting for button press...
✅ Authentication successful!
🎉 You can now use huestatus!
💥 Internal error: panicked at /rustc/283db70ace62a0ae704a624e43b68c2ee44b87a6/library/alloc/src/slice.rs:525:50:
capacity overflow
Please report this issue at: https://github.com/mimikun/huestatus/issues
```

エラー内容は以下の通りです.

```txt
💥 Internal error: panicked at /rustc/283db70ace62a0ae704a624e43b68c2ee44b87a6/library/alloc/src/slice.rs:525:50:
capacity overflow
```
