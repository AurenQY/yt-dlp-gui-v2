export const CLIENT_PROTOCOL = Object.freeze({
  wireVersion: 1,
  capabilities: Object.freeze([
    'fields.ignore-unknown.v1',
    'events.ignore-unknown.v1',
  ]),
});

export function withClientProtocol(request) {
  return {
    ...request,
    client: {
      wireVersion: CLIENT_PROTOCOL.wireVersion,
      capabilities: [...CLIENT_PROTOCOL.capabilities],
    },
  };
}

export function hostProtocolFrom(data) {
  const protocol = data?.protocol;
  if (!protocol || typeof protocol !== 'object') {
    return { advertised: false, wireVersion: null, capabilities: [], limits: {} };
  }
  return {
    advertised: true,
    wireVersion: Number.isInteger(protocol.wireVersion) ? protocol.wireVersion : null,
    capabilities: Array.isArray(protocol.capabilities)
      ? protocol.capabilities.filter(value => typeof value === 'string')
      : [],
    limits: protocol.limits && typeof protocol.limits === 'object' ? { ...protocol.limits } : {},
  };
}

export function hostSupports(data, capability) {
  return protocolSupports(hostProtocolFrom(data), capability);
}

export function protocolSupports(protocol, capability) {
  return Array.isArray(protocol?.capabilities) && protocol.capabilities.includes(capability);
}
