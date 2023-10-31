package org.mozilla.geckoview;

import org.mozilla.gecko.util.GeckoBundle;

public class TorConnectError {
    public String code;
    public String message;
    public String phase;
    public String reason;

    public TorConnectError(GeckoBundle bundle) {
        code = bundle.getString("code");
        message = bundle.getString("message");
        phase = bundle.getString("phase");
        reason = bundle.getString("reason");
    }

    public TorConnectError(String code, String message, String phase, String reason) {
        this.code = code;
        this.message = message;
        this.phase = phase;
        this.reason = reason;
    }
}
