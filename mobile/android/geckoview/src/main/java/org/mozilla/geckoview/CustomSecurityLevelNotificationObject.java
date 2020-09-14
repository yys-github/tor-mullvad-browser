package org.mozilla.geckoview;

import org.mozilla.gecko.util.EventCallback;

public class CustomSecurityLevelNotificationObject {
  public boolean isCustom;
  public EventCallback callback;

  public CustomSecurityLevelNotificationObject(boolean isCustom, EventCallback callback) {
    this.isCustom = isCustom;
    this.callback = callback;
  }
}
