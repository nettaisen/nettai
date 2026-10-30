package nettai.platform;

import android.app.Activity;
import android.app.Fragment;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
import android.util.Log;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

/**
 * nettai's dialogs on Android (crates/nettai-platform/src/dialog/phone/android.rs): the system's pickers and its
 * save dialog, each started by a fragment of its own added to the activity, which hears its result; and the
 * clipboard. The app's native library calls the static methods through JNI (the class found by the activity's class
 * loader) and hears the answers through the native methods it registers. Part of the APK: its build compiles this.
 */
public final class NettaiFiles extends Fragment {
    private static final String TAG = "nettai";
    private static final int FILES = 1, TREE = 2, SAVE = 3;

    /** Files picked: a line each, "name\turi"; null if none were. */
    static native void picked(long request, String listing);

    /** A folder picked: its files a line each, "path/in/it\turi", as deep as asked; null if none was picked. */
    static native void listed(long request, String listing);

    private long request;
    private int kind;
    private int depth;
    private String source;
    private String name;

    public NettaiFiles() {}

    /** Files to open, any number of them. */
    public static void openFiles(Activity activity, long request) {
        start(activity, request, FILES, 0, null, null);
    }

    /** A folder to open: its files listed {@code depth} folders deep. */
    public static void openTree(Activity activity, long request, int depth) {
        start(activity, request, TREE, depth, null, null);
    }

    /** The file at {@code source} saved where the player says, as {@code name}; the file is removed after. */
    public static void save(Activity activity, String source, String name) {
        start(activity, 0, SAVE, 0, source, name);
    }

    /** Text copied to the clipboard. */
    public static void copyText(final Activity activity, final String text) {
        activity.runOnUiThread(new Runnable() {
            public void run() {
                ClipboardManager clipboard = (ClipboardManager) activity.getSystemService(Context.CLIPBOARD_SERVICE);
                clipboard.setPrimaryClip(ClipData.newPlainText("nettai", text));
            }
        });
    }

    /** A document's bytes copied to the file at {@code to} (the native library calls this on a thread of its own). */
    public static void copy(Context context, String uri, String to) throws IOException {
        InputStream in = context.getContentResolver().openInputStream(Uri.parse(uri));
        if (in == null) throw new IOException("no stream for " + uri);
        try {
            OutputStream out = new FileOutputStream(to);
            try {
                pipe(in, out);
            } finally {
                out.close();
            }
        } finally {
            in.close();
        }
    }

    private static void start(final Activity activity, final long request, final int kind, final int depth, final String source, final String name) {
        activity.runOnUiThread(new Runnable() {
            public void run() {
                NettaiFiles fragment = new NettaiFiles();
                fragment.request = request;
                fragment.kind = kind;
                fragment.depth = depth;
                fragment.source = source;
                fragment.name = name;
                activity.getFragmentManager().beginTransaction().add(fragment, "nettai-files-" + kind + "-" + request).commitAllowingStateLoss();
            }
        });
    }

    @Override
    public void onCreate(Bundle state) {
        super.onCreate(state);
        // (Made again with the activity: the system's dialog is up already, and its result comes here.)
        if (state != null) {
            request = state.getLong("request");
            kind = state.getInt("kind");
            depth = state.getInt("depth");
            source = state.getString("source");
            name = state.getString("name");
            return;
        }
        Intent intent;
        if (kind == FILES) {
            intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType("*/*");
            intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
        } else if (kind == TREE) {
            intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
        } else {
            intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType(name.toLowerCase().endsWith(".mp4") ? "video/mp4" : "application/octet-stream");
            intent.putExtra(Intent.EXTRA_TITLE, name);
        }
        startActivityForResult(intent, kind);
    }

    @Override
    public void onSaveInstanceState(Bundle state) {
        super.onSaveInstanceState(state);
        state.putLong("request", request);
        state.putInt("kind", kind);
        state.putInt("depth", depth);
        state.putString("source", source);
        state.putString("name", name);
    }

    @Override
    public void onActivityResult(int code, int result, Intent data) {
        final Activity activity = getActivity();
        getFragmentManager().beginTransaction().remove(this).commitAllowingStateLoss();
        final Uri chosen = result == Activity.RESULT_OK && data != null ? data.getData() : null;
        if (kind == FILES) {
            if (result != Activity.RESULT_OK || data == null) {
                picked(request, null);
                return;
            }
            StringBuilder out = new StringBuilder();
            ClipData many = data.getClipData();
            if (many != null) {
                for (int i = 0; i < many.getItemCount(); i++) line(out, nameOf(activity, many.getItemAt(i).getUri()), many.getItemAt(i).getUri());
            } else if (chosen != null) {
                line(out, nameOf(activity, chosen), chosen);
            }
            picked(request, out.toString());
        } else if (kind == TREE) {
            if (chosen == null) {
                listed(request, null);
                return;
            }
            final long asked = request;
            final int deep = depth;
            new Thread(new Runnable() {
                public void run() {
                    StringBuilder out = new StringBuilder();
                    walk(activity.getContentResolver(), chosen, DocumentsContract.getTreeDocumentId(chosen), "", deep, out);
                    listed(asked, out.toString());
                }
            }, "nettai-tree").start();
        } else {
            final File from = new File(source);
            if (chosen == null) {
                from.delete();
                return;
            }
            new Thread(new Runnable() {
                public void run() {
                    try {
                        InputStream in = new FileInputStream(from);
                        try {
                            OutputStream out = activity.getContentResolver().openOutputStream(chosen);
                            if (out == null) throw new IOException("no stream for " + chosen);
                            try {
                                pipe(in, out);
                            } finally {
                                out.close();
                            }
                        } finally {
                            in.close();
                        }
                    } catch (IOException e) {
                        Log.w(TAG, "saving " + from + ": " + e);
                    }
                    from.delete();
                }
            }, "nettai-save").start();
        }
    }

    /** The files under {@code document} of the tree, a line each, as deep as {@code left} folders. */
    private static void walk(ContentResolver resolver, Uri tree, String document, String at, int left, StringBuilder out) {
        Uri children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, document);
        String[] columns = {DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME, DocumentsContract.Document.COLUMN_MIME_TYPE};
        Cursor rows = resolver.query(children, columns, null, null, null);
        if (rows == null) return;
        try {
            while (rows.moveToNext()) {
                String id = rows.getString(0);
                String name = clean(rows.getString(1));
                String path = at.isEmpty() ? name : at + "/" + name;
                if (DocumentsContract.Document.MIME_TYPE_DIR.equals(rows.getString(2))) {
                    if (left > 1) walk(resolver, tree, id, path, left - 1, out);
                } else {
                    line(out, path, DocumentsContract.buildDocumentUriUsingTree(tree, id));
                }
            }
        } finally {
            rows.close();
        }
    }

    private static String nameOf(Activity activity, Uri uri) {
        Cursor rows = activity.getContentResolver().query(uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null);
        if (rows != null) {
            try {
                if (rows.moveToFirst() && rows.getString(0) != null) return rows.getString(0);
            } finally {
                rows.close();
            }
        }
        return uri.getLastPathSegment();
    }

    /** A name as a line holds it: no tab, line break or folder separator. */
    private static String clean(String name) {
        return name == null ? "" : name.replace('\t', ' ').replace('\n', ' ').replace('/', ' ');
    }

    private static void line(StringBuilder out, String name, Uri uri) {
        out.append(clean(name)).append('\t').append(uri.toString()).append('\n');
    }

    private static void pipe(InputStream in, OutputStream out) throws IOException {
        byte[] buffer = new byte[1 << 16];
        int n;
        while ((n = in.read(buffer)) > 0) out.write(buffer, 0, n);
    }
}
