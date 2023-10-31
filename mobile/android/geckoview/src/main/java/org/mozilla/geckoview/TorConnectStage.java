package org.mozilla.geckoview;

import org.mozilla.gecko.util.GeckoBundle;

// Class to receive ConnectStage object from TorConnect.sys.mjs ~ln677
public class TorConnectStage {

    public TorConnectStageName name;
    // The TorConnectStage prior to this bootstrap attempt. Only set during the "Bootstrapping" stage.
    public TorConnectStageName bootstrapTrigger;
    public TorConnectError error;
    public String defaultRegion;
    public Boolean potentiallyBlocked;
    public Boolean tryAgain;
    public TorBootstrappingStatus bootstrappingStatus;

    public TorConnectStage(GeckoBundle bundle) {
        name = TorConnectStageName.fromString(bundle.getString("name"));
        if (bundle.getString("bootstrapTrigger") != null) {
            bootstrapTrigger = TorConnectStageName.fromString(bundle.getString("bootstrapTrigger"));
        }
        defaultRegion = bundle.getString("defaultRegion");
        potentiallyBlocked = bundle.getBoolean("potentiallyBlocked");
        tryAgain = bundle.getBoolean("tryAgain");
        if (bundle.getBundle("error") != null) {
            error = new TorConnectError(bundle.getBundle("error"));
        }
        bootstrappingStatus = new TorBootstrappingStatus(bundle.getBundle("bootstrappingStatus"));
    }

    public Boolean isBootstrapped() {
        return name.isBootstrapped();
    }
}
