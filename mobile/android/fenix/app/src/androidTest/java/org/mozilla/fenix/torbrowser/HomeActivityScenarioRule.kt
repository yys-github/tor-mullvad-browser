/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.torbrowser

import androidx.test.ext.junit.rules.ActivityScenarioRule
import org.junit.rules.TestRule
import org.junit.runner.Description
import org.junit.runners.model.Statement
import org.mozilla.fenix.HomeActivity
import org.mozilla.fenix.ext.components

class HomeActivityScenarioRule : TestRule {
    private val delegate = ActivityScenarioRule<HomeActivity>(HomeActivity::class.java)

    val scenario get() = delegate.scenario

    // ActivityScenarioRule waits for the DESTROYED lifecycle state during teardown, which means
    // HomeActivity.onDestroy runs synchronously before the rule returns. onDestroy calls shutDown() ->
    // exitProcess(0) when isFinishing, which kills the instrumentation process. Setting this flag
    // prevents that code path so tests can report their results normally.
    override fun apply(base: Statement, description: Description): Statement {
        return delegate.apply(object : Statement() {
            override fun evaluate() {
                delegate.scenario.onActivity { activity ->
                    activity.applicationContext.components.notificationsDelegate.shouldShutDownWithOnDestroyWhenIsFinishing = false
                }
                base.evaluate()
            }
        }, description)
    }
}
