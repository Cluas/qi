package eu.org.cluas.qi

import android.os.Bundle
import androidx.activity.ComponentActivity
// import dev.nsh.NshView

/**
 * 「知气 · Qi」Android 原生宿主入口（搭配 nsh.aar 使用）：
 * V1 作为拍照饮食 / 体检离线建档 / WebDAV / BYO-Server / 局域网 Ollama 伴侣壳，
 * Health Connect 与华为 Health Kit 同步游标通过 NshView host bridge 直接落盘。
 */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // val nshView = NshView(this)
        // nshView.loadBundledApp("bundle.nshc")
        // setContentView(nshView)
    }
}
