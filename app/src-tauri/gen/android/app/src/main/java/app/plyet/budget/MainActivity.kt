package app.plyet.budget

import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    webView.settings.setSupportZoom(false)
    webView.settings.builtInZoomControls = false
    webView.settings.displayZoomControls = false
    webView.isHorizontalScrollBarEnabled = false
    webView.isVerticalScrollBarEnabled = false
    webView.overScrollMode = View.OVER_SCROLL_NEVER

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
      insets // observe only, don't consume
    }
  }
}
