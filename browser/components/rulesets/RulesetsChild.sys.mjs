// Copyright (c) 2022, The Tor Project, Inc.

import { RemotePageChild } from "moz-src:///toolkit/actors/RemotePageChild.sys.mjs";

/**
 * The child actor part for about:rulesets.
 * It does not do anything, as all the communication happens with RPM* calls.
 */
export class RulesetsChild extends RemotePageChild {}
