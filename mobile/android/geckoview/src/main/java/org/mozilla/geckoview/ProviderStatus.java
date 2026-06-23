package org.mozilla.geckoview;

import org.mozilla.gecko.util.GeckoBundle;

public class ProviderStatus {
  public Boolean maybeConfigIssue;

  public ProviderStatus(GeckoBundle bundle) {
    maybeConfigIssue = bundle.getBoolean("maybeConfigIssue");
  }
}
