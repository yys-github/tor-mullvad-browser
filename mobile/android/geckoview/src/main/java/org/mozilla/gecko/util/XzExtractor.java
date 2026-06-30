package org.mozilla.gecko.util;

import android.content.Context;
import android.content.res.AssetManager;
import android.util.Log;
import java.io.BufferedReader;
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import org.tukaani.xz.XZInputStream;

public class XzExtractor {
  private static final String TAG = "XzExtractor";
  private final Context mContext;
  private final String mSource;
  private String mSourceHash;
  private final File mTarget;
  private final File mTargetHash;

  public XzExtractor(Context context, String source, File targetDir, String destination) {
    mContext = context;
    mSource = source;
    mTarget = new File(targetDir, destination);
    mTargetHash = new File(targetDir, destination + ".sha256");
  }

  public File extract() throws IOException {
    Log.d(TAG, "Extracting " + mSource + " -> " + mTarget.getAbsolutePath());
    readSourceTag();
    if (checkExisting()) {
      Log.d(TAG, mSource + ": up to date, skipping extraction");
    } else {
      Log.d(TAG, mSource + ": extracting from assets");
      extractFromAssets();
      Log.d(TAG, mSource + ": extraction complete");
    }
    return mTarget;
  }

  private void readSourceTag() throws IOException {
    // We use sha256 because we need a unique deterministic string
    // to verify if current resources are up to date or need to be
    // re-extracted. THIS IS NOT AN INTEGRITY CHECK.
    final String asset = mSource + ".sha256";
    final Context context = mContext;
    try (InputStream in = context.getAssets().open(asset, AssetManager.ACCESS_BUFFER);
         BufferedReader reader = new BufferedReader(new InputStreamReader(in, StandardCharsets.UTF_8))) {
      mSourceHash = reader.readLine();
    }
    if (mSourceHash == null || mSourceHash.isEmpty()) {
      throw new IOException(asset + " is empty");
    }
  }

  private boolean checkExisting() throws IOException {
    if (!mTarget.exists() || !mTargetHash.exists()) {
      return false;
    }
    String destHash = new String(Files.readAllBytes(mTargetHash.toPath()), StandardCharsets.UTF_8).trim();
    return destHash.equals(mSourceHash);
  }

  private void extractFromAssets() throws IOException {
    final Context context = mContext;
    try (InputStream assetIn = context.getAssets().open(mSource + ".xz");
        XZInputStream xzIn = new XZInputStream(assetIn)) {
      Files.copy(xzIn, mTarget.toPath(), StandardCopyOption.REPLACE_EXISTING);
    }
    Files.write(mTargetHash.toPath(), mSourceHash.getBytes(StandardCharsets.UTF_8));
  }
}
