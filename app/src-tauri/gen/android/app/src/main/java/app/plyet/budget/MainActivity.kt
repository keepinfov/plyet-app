package app.plyet.budget

import android.os.Bundle
import android.view.View
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

class MainActivity : TauriActivity() {
  private var webViewRef: WebView? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    hideStatusBar()
  }

  // Hide the status bar in immersive mode. With BEHAVIOR_SHOW_TRANSIENT_BARS_BY_
  // SWIPE the bar reappears as a translucent overlay on a top-edge swipe, then
  // auto-hides — the behavior most fullscreen apps use. The nav bar is left
  // alone (gesture nav stays usable).
  private fun hideStatusBar() {
    val controller = WindowCompat.getInsetsController(window, window.decorView)
    controller.systemBarsBehavior =
      WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    controller.hide(WindowInsetsCompat.Type.statusBars())
  }

  // The system restores bars after losing focus (dialogs, recents, IME). Re-hide
  // once we regain focus so the immersive state sticks.
  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    if (hasFocus) hideStatusBar()
  }

  // Match the transient status bar's icon tint to the app theme so the clock /
  // battery / icons stay legible: dark theme → light (white) icons; light theme
  // → dark icons. isAppearanceLightStatusBars=true means "light background" →
  // dark icons, so it's the inverse of isDark.
  private fun setStatusBarDark(isDark: Boolean) {
    runOnUiThread {
      WindowCompat.getInsetsController(window, window.decorView)
        .isAppearanceLightStatusBars = !isDark
    }
  }

  private inner class ThemeBridge {
    @JavascriptInterface
    fun setStatusBarDark(isDark: Boolean) = this@MainActivity.setStatusBarDark(isDark)

    // JS → native: force a fresh inset dispatch. The OnApplyWindowInsetsListener
    // fires once during initial layout — before the web page has registered
    // window.__plyetSetInsets — so that first emission is lost and insets only
    // appear after the next window change. The web inset controller calls this
    // on init (bridge now ready) to pull the current values immediately.
    @JavascriptInterface
    fun requestInsets() {
      val wv = webViewRef ?: return
      wv.post { ViewCompat.requestApplyInsets(wv) }
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    webViewRef = webView
    webView.settings.setSupportZoom(false)
    webView.settings.builtInZoomControls = false
    webView.settings.displayZoomControls = false
    webView.isHorizontalScrollBarEnabled = false
    webView.isVerticalScrollBarEnabled = false
    webView.overScrollMode = View.OVER_SCROLL_NEVER

    // JS → native: let the web theme controller tint the transient status bar.
    webView.addJavascriptInterface(ThemeBridge(), "PlyetNative")

    // Native IME-inset bridge. enableEdgeToEdge() defeats the manifest's
    // adjustResize, so the WebView never shrinks for the keyboard; older /
    // no-GMS WebViews (Huawei) also lack interactive-widget, so the web-side
    // visualViewport path is a no-op. Read the real IME height from the Android
    // framework and push it (in CSS px) into the web keyboard controller.
    ViewCompat.setOnApplyWindowInsetsListener(webView) { _, insets ->
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      val nav = insets.getInsets(WindowInsetsCompat.Type.navigationBars()).bottom
      val px = maxOf(0, ime - nav) // keyboard height above the nav bar
      val dp = px / webView.resources.displayMetrics.density
      webView.evaluateJavascript(
        "window.__plyetSetKeyboardInset && window.__plyetSetKeyboardInset($dp)",
        null
      )

      // System-bar insets the WebView's env(safe-area-inset-*) can't see under
      // edge-to-edge: status bar is hidden (env reports cutout only) and the nav
      // bar height isn't reflected. Push framework-measured values so layout can
      // reserve real space. Top uses IgnoringVisibility so the reserve stays
      // stable while the transient status bar swipes in/out.
      val density = webView.resources.displayMetrics.density
      val statusTop = maxOf(
        insets.getInsetsIgnoringVisibility(WindowInsetsCompat.Type.statusBars()).top,
        insets.getInsets(WindowInsetsCompat.Type.displayCutout()).top
      )
      val navBottom = insets.getInsets(WindowInsetsCompat.Type.navigationBars()).bottom
      val topDp = statusTop / density
      val bottomDp = navBottom / density
      webView.evaluateJavascript(
        "window.__plyetSetInsets && window.__plyetSetInsets($topDp,$bottomDp)",
        null
      )
      insets // observe only, don't consume
    }
  }
}
