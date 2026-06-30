package org.mozilla.gecko.util;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import android.content.Context;
import android.test.suitebuilder.annotation.SmallTest;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.robolectric.RobolectricTestRunner;
import org.robolectric.RuntimeEnvironment;

@RunWith(RobolectricTestRunner.class)
@SmallTest
public class XzExtractorTest {
  private Context mContext;
  private File mTargetDir;

  private String mTestFileContents = "#!/bin/sh\necho hello world";

  @Before
  public void setUp() throws IOException {
    mContext = RuntimeEnvironment.getApplication();
    mTargetDir = Files.createTempDirectory("xz-extractor-test").toFile();
  }

  @After
  public void tearDown() {
    deleteRecursively(mTargetDir);
  }

  @Test
  public void freshExtraction() throws IOException, InterruptedException {
    XzExtractor extractor = new XzExtractor(mContext, "test", mTargetDir, "test");
    File extracted = extractor.extract();

    assertTrue(extracted.exists());
    assertTrue(new File(mTargetDir, "test.sha256").exists());
  }

  @Test
  public void upToDateSkipsExtraction() throws IOException {
    XzExtractor extractor = new XzExtractor(mContext, "test", mTargetDir, "test");
    File extracted = extractor.extract();
    assertEquals(
        mTestFileContents,
        new String(Files.readAllBytes(extracted.toPath()), StandardCharsets.UTF_8).trim());

    Files.write(extracted.toPath(), "tampered".getBytes(StandardCharsets.UTF_8));
    extractor.extract();

    assertEquals(
        "tampered",
        new String(Files.readAllBytes(extracted.toPath()), StandardCharsets.UTF_8));
  }

  @Test
  public void staleHashTriggersReExtraction() throws IOException, InterruptedException {
    XzExtractor extractor = new XzExtractor(mContext, "test", mTargetDir, "test");
    File extracted = extractor.extract();
    File hashFile = new File(mTargetDir, "test.sha256");

    assertEquals(
        mTestFileContents,
        new String(Files.readAllBytes(extracted.toPath()), StandardCharsets.UTF_8).trim());

    Files.write(hashFile.toPath(), "new-hash".getBytes(StandardCharsets.UTF_8));
    Files.write(extracted.toPath(), "tampered".getBytes(StandardCharsets.UTF_8));

    extractor.extract();
    assertEquals(
        mTestFileContents,
        new String(Files.readAllBytes(extracted.toPath()), StandardCharsets.UTF_8).trim());
  }

  @Test
  public void missingAssetThrows() {
    XzExtractor extractor = new XzExtractor(mContext, "nonexistent", mTargetDir, "test");
    assertThrows(IOException.class, extractor::extract);
  }

  @Test
  public void missingHashFileThrows() {
    XzExtractor extractor = new XzExtractor(mContext, "no-hash", mTargetDir, "test");
    assertThrows(IOException.class, extractor::extract);
  }

  @Test
  public void emptyHashFileThrows() {
    XzExtractor extractor = new XzExtractor(mContext, "empty-hash", mTargetDir, "test");
    assertThrows(IOException.class, extractor::extract);
  }

  private void deleteRecursively(final File file) {
    if (file.isDirectory()) {
      final File[] children = file.listFiles();
      if (children != null) {
        for (final File child : children) {
          deleteRecursively(child);
        }
      }
    }
    file.delete();
  }
}
