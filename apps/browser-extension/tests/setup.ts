import '@testing-library/jest-dom';

// Mock browser APIs
global.browser = {
  runtime: {
    id: 'test-extension-id',
    getManifest: () => ({ version: '0.1.0' }),
    sendMessage: jest.fn(),
    onMessage: { addListener: jest.fn(), removeListener: jest.fn() },
  },
  storage: {
    local: {
      get: jest.fn(),
      set: jest.fn(),
      remove: jest.fn(),
      clear: jest.fn(),
    },
    sync: {
      get: jest.fn(),
      set: jest.fn(),
      remove: jest.fn(),
      clear: jest.fn(),
    },
    onChanged: { addListener: jest.fn(), removeListener: jest.fn() },
  },
  downloads: {
    download: jest.fn(),
    search: jest.fn(),
  },
  notifications: {
    create: jest.fn(),
  },
  tabs: {
    create: jest.fn(),
    query: jest.fn(),
    sendMessage: jest.fn(),
  },
  contextMenus: {
    create: jest.fn(),
    onClicked: { addListener: jest.fn() },
  },
  permissions: {
    contains: jest.fn(),
    request: jest.fn(),
  },
};

// Mock fetch
global.fetch = jest.fn();

// Reset mocks between tests
beforeEach(() => {
  jest.clearAllMocks();
});
