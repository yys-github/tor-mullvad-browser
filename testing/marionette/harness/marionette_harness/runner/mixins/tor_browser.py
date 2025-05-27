from marionette_driver import Wait
from marionette_driver.errors import ScriptTimeoutException

DEFAULT_BOOTSTRAP_TIMEOUT_MS = 60 * 1000
DEFAULT_BOOTSTRAP_MAX_RETRIES = 3

class TorBrowserMixin:
    def bootstrap(
        self,
        max_retries=DEFAULT_BOOTSTRAP_MAX_RETRIES,
    ):
        """Bootstrap the Tor connection.

        This doesn't fail if already bootstrapped, but will retry a few times if
        a script timeout is hit.

        This function is UI-agnostic, meaning it can be used both on Desktop and Android.
        """

        attempt = 0
        while attempt < max_retries:
            try:
                with self.marionette.using_context("chrome"):
                    did_bootstrap = self.marionette.execute_async_script(
                        """
                        const { TorConnect, TorConnectStage, TorConnectTopics } = ChromeUtils.importESModule(
                            "resource://gre/modules/TorConnect.sys.mjs"
                        );
                        const [resolve] = arguments;

                        // Only the first test of a suite will need to bootstrap.
                        if (TorConnect.stage.name === TorConnectStage.Bootstrapped) {
                            resolve(false);
                            return;
                        }

                        function waitForBootstrap() {
                            const topic = TorConnectTopics.BootstrapComplete;
                            Services.obs.addObserver(function observer() {
                                Services.obs.removeObserver(observer, topic);
                                resolve(true);
                            }, topic);
                            TorConnect.beginBootstrapping();
                        }

                        const stageTopic = TorConnectTopics.StageChange;
                        function stageObserver() {
                            if (TorConnect.canBeginNormalBootstrap) {
                                Services.obs.removeObserver(stageObserver, stageTopic);
                                waitForBootstrap();
                            }
                        }
                        Services.obs.addObserver(stageObserver, stageTopic);
                        stageObserver();
                        """,
                        script_timeout=DEFAULT_BOOTSTRAP_TIMEOUT_MS,
                    )

                # The above script waits for bootstrap to be complete,
                # but doesn't wait for the redirection to about:blank that
                # happens after bootstrap to be complete.
                #
                # We need to wait for this navigation to complete,
                # otherwise subsequent calls to navigate may race with it.
                #
                # Android doesn't do any redirection, the tor connect UI in
                # there is native and the initial state of the browser
                # doesn't even have an open tab to check against.
                # So we skip this check for that platform.
                if did_bootstrap and self.marionette.session_capabilities.get("browserName") != "fennec":
                    Wait(self.marionette).until(
                        lambda mn: mn.get_url() == "about:blank",
                        message="Still not in about:blank",
                    )

                return
            except ScriptTimeoutException:
                attempt += 1
                with self.marionette.using_context("chrome"):
                    self.marionette.execute_script(
                        """
                        const { TorConnect } = ChromeUtils.importESModule(
                            "resource://gre/modules/TorConnect.sys.mjs"
                        );

                        TorConnect._makeStageRequest(TorConnectStage.Start, true);
                        """
                    )


        raise RuntimeError("Unable to connect to Tor Network")
