export ANDROID_HOME=$HOME/Android/Sdk;
export NDK_VERSION=$(ls $ANDROID_HOME/ndk | head -1);
export ANDROID_NDK=$ANDROID_HOME/ndk/$NDK_VERSION;
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk;
export ANDROID_NDK_HOME=$ANDROID_NDK