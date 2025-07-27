commit `f010dafa1d804fa46c0f7fdf8f11a34d3bb65967` で修正してもらった認証エラーですが、まだ発生します。

`#` でコメントを入れています。実際の出力には `#` ではじまる表示はありませんので注意してください。

```txt
🏗️  Huestatus Setup
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Welcome to huestatus! Let's configure your Philips Hue lights.

⚙️ Step 1/7: Initializing setup...

🔍 Step 2/7: Discovering Hue bridges...

🔑 Step 3/7: Authenticating with bridge at 192.168.1.146...

🔑 Authentication Required

To authenticate with your Hue bridge:
1. Press the large button on top of your Hue bridge
2. Wait for the button to start blinking
3. Press Enter to continue
# ここでEnterキーを押し、すかさずブリッジの丸いボタンを押す

Press the bridge button now and then press Enter...🔍 Attempting authentication...
⏳ Waiting for button press...
✅ Authentication successful!
🎉 You can now use huestatus!
💥 Internal error: panicked at src/setup/mod.rs:242:36:
attempt to subtract with overflow
Please report this issue at: https://github.com/mimikun/huestatus/issues
```
