// Copyright (c) 2020, The Tor Project, Inc.

/**
 * This class contains the child part of Onion Location.
 */
export class OnionLocationChild extends JSWindowActorChild {
  handleEvent(event) {
    this.onPageShow(event);
  }

  onPageShow(event) {
    if (event.target != this.document) {
      return;
    }
    const onionLocationURI = this.document.onionLocationURI;
    if (onionLocationURI) {
      this.sendAsyncMessage("OnionLocation:Set", {
        uri: onionLocationURI.asciiSpec,
      });
    }
  }
}
