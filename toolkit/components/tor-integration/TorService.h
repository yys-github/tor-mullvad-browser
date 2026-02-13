/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifndef TOOLKIT_COMPONENTS_TOR_INTEGRATION_TORSERVICE_H_
#define TOOLKIT_COMPONENTS_TOR_INTEGRATION_TORSERVICE_H_

#include "nsCOMPtr.h"

#include "ITorService.h"

// Inspired by
// toolkit/components/extensions/storage/ExtensionStorageComponents.h.

// Implemented in Rust
extern "C" nsresult NewTorServiceImpl(ITorService** aResult);

namespace torproject {
already_AddRefed<ITorService> NewTorService() {
  nsCOMPtr<ITorService> service;
  nsresult rv = NewTorServiceImpl(getter_AddRefs(service));
  if (NS_WARN_IF(NS_FAILED(rv))) {
    return nullptr;
  }
  return service.forget();
}
}  // namespace torproject

#endif  // TOOLKIT_COMPONENTS_TOR_INTEGRATION_TORSERVICE_H_
