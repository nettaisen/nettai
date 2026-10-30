// (android.app.Fragment is deprecated and still in every Android: it is what hears an activity result without
// AndroidX.)
@file:Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")

package nettai.dialogs

import android.app.Activity
import android.app.Fragment
import android.content.ClipData
import android.content.ClipboardManager
import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import android.util.Log
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream

/**
 * nettai's dialogs on Android (crates/nettai-dialogs/src/platform/android.rs): the system's pickers and its save
 * dialog, each started by a fragment of its own added to the activity, which hears its result; and the clipboard.
 * The app's native library calls the companion's functions through JNI (static, @JvmStatic; the class found by
 * the activity's class loader) and hears the answers through the `external` functions it registers. Part of the
 * APK: packaging/android/package.sh compiles it.
 */
class NettaiFiles : Fragment() {
    private var request = 0L
    private var kind = 0
    private var depth = 0
    private var source: String? = null
    private var name: String? = null

    companion object {
        private const val TAG = "nettai"
        private const val FILES = 1
        private const val TREE = 2
        private const val SAVE = 3

        /** Files picked: a line each, "name\turi"; null if none were. */
        @JvmStatic external fun picked(request: Long, listing: String?)

        /** A folder picked: its files a line each, "path/in/it\turi", as deep as asked; null if none was picked. */
        @JvmStatic external fun listed(request: Long, listing: String?)

        /** Files to open, any number of them. */
        @JvmStatic
        fun openFiles(activity: Activity, request: Long) = start(activity, request, FILES, 0, null, null)

        /** A folder to open: its files listed [depth] folders deep. */
        @JvmStatic
        fun openTree(activity: Activity, request: Long, depth: Int) = start(activity, request, TREE, depth, null, null)

        /** The file at [source] saved where the player says, as [name]; the file is removed after. */
        @JvmStatic
        fun save(activity: Activity, source: String, name: String) = start(activity, 0, SAVE, 0, source, name)

        /** Text copied to the clipboard. */
        @JvmStatic
        fun copyText(activity: Activity, text: String) {
            activity.runOnUiThread {
                val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                clipboard.setPrimaryClip(ClipData.newPlainText("nettai", text))
            }
        }

        /** A document's bytes copied to the file at [to] (the native library calls this on a thread of its own). */
        @JvmStatic
        @Throws(IOException::class)
        fun copy(context: Context, uri: String, to: String) {
            val input = context.contentResolver.openInputStream(Uri.parse(uri)) ?: throw IOException("no stream for $uri")
            input.use { inp -> FileOutputStream(to).use { out -> pipe(inp, out) } }
        }

        private fun start(activity: Activity, request: Long, kind: Int, depth: Int, source: String?, name: String?) {
            activity.runOnUiThread {
                val fragment = NettaiFiles()
                fragment.request = request
                fragment.kind = kind
                fragment.depth = depth
                fragment.source = source
                fragment.name = name
                activity.fragmentManager.beginTransaction().add(fragment, "nettai-files-$kind-$request").commitAllowingStateLoss()
            }
        }

        /** The files under [document] of the tree, a line each, as deep as [left] folders. */
        private fun walk(resolver: ContentResolver, tree: Uri, document: String, at: String, left: Int, out: StringBuilder) {
            val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, document)
            val columns = arrayOf(
                DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                DocumentsContract.Document.COLUMN_MIME_TYPE,
            )
            resolver.query(children, columns, null, null, null)?.use { rows ->
                while (rows.moveToNext()) {
                    val id = rows.getString(0)
                    val name = clean(rows.getString(1))
                    val path = if (at.isEmpty()) name else "$at/$name"
                    if (DocumentsContract.Document.MIME_TYPE_DIR == rows.getString(2)) {
                        if (left > 1) walk(resolver, tree, id, path, left - 1, out)
                    } else {
                        line(out, path, DocumentsContract.buildDocumentUriUsingTree(tree, id))
                    }
                }
            }
        }

        private fun nameOf(activity: Activity, uri: Uri): String {
            activity.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { rows ->
                if (rows.moveToFirst()) rows.getString(0)?.let { return it }
            }
            return uri.lastPathSegment ?: ""
        }

        /** A name as a line holds it: no tab, line break or folder separator. */
        private fun clean(name: String?): String = (name ?: "").replace('\t', ' ').replace('\n', ' ').replace('/', ' ')

        private fun line(out: StringBuilder, name: String, uri: Uri) {
            out.append(clean(name)).append('\t').append(uri.toString()).append('\n')
        }

        private fun pipe(input: InputStream, output: OutputStream) {
            val buffer = ByteArray(1 shl 16)
            while (true) {
                val n = input.read(buffer)
                if (n <= 0) break
                output.write(buffer, 0, n)
            }
        }
    }

    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        // (Made again with the activity: the system's dialog is up already, and its result comes here.)
        if (state != null) {
            request = state.getLong("request")
            kind = state.getInt("kind")
            depth = state.getInt("depth")
            source = state.getString("source")
            name = state.getString("name")
            return
        }
        val intent = when (kind) {
            FILES -> Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
                putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
            }
            TREE -> Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
            else -> Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = if (name.orEmpty().lowercase().endsWith(".mp4")) "video/mp4" else "application/octet-stream"
                putExtra(Intent.EXTRA_TITLE, name)
            }
        }
        startActivityForResult(intent, kind)
    }

    override fun onSaveInstanceState(state: Bundle) {
        super.onSaveInstanceState(state)
        state.putLong("request", request)
        state.putInt("kind", kind)
        state.putInt("depth", depth)
        state.putString("source", source)
        state.putString("name", name)
    }

    override fun onActivityResult(code: Int, result: Int, data: Intent?) {
        val activity = activity
        fragmentManager.beginTransaction().remove(this).commitAllowingStateLoss()
        val chosen = if (result == Activity.RESULT_OK) data?.data else null
        when (kind) {
            FILES -> {
                if (result != Activity.RESULT_OK || data == null) return picked(request, null)
                val out = StringBuilder()
                val many = data.clipData
                if (many != null) {
                    for (i in 0 until many.itemCount) line(out, nameOf(activity, many.getItemAt(i).uri), many.getItemAt(i).uri)
                } else if (chosen != null) {
                    line(out, nameOf(activity, chosen), chosen)
                }
                picked(request, out.toString())
            }
            TREE -> {
                if (chosen == null) return listed(request, null)
                val (asked, deep) = request to depth
                Thread({
                    val out = StringBuilder()
                    walk(activity.contentResolver, chosen, DocumentsContract.getTreeDocumentId(chosen), "", deep, out)
                    listed(asked, out.toString())
                }, "nettai-tree").start()
            }
            else -> {
                val from = File(source ?: return)
                if (chosen == null) {
                    from.delete()
                    return
                }
                Thread({
                    try {
                        FileInputStream(from).use { input ->
                            val output = activity.contentResolver.openOutputStream(chosen) ?: throw IOException("no stream for $chosen")
                            output.use { pipe(input, it) }
                        }
                    } catch (e: IOException) {
                        Log.w(TAG, "saving $from: $e")
                    }
                    from.delete()
                }, "nettai-save").start()
            }
        }
    }
}
