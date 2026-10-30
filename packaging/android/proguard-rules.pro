# What native code calls by name through JNI, which R8 can't see: the app's Kotlin (nettai-dialogs' NettaiFiles,
# nettai-lifecycle's NettaiAwake, nettai-gamepad's NettaiGamepads and their native methods), GameActivity's (its C calls
# into it), and the certificate verifier's (rustls-platform-verifier's Kotlin, which reqwest's checks call).
-keep class nettai.** { *; }
-keep class com.google.androidgamesdk.** { *; }
-keep class org.rustls.platformverifier.** { *; }
