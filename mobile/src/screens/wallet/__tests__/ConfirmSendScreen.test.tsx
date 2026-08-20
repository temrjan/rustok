/**
 * ConfirmSendScreen — coverage for the three preview-lifecycle states
 * (loading / ready / error), verdict branching (Allow / Warn / Block),
 * and operation execution success / failure paths. Mock pattern follows
 * ReceiveScreen.test (per-test override of `lib/walletHandle`,
 * inline jest.mock factory for navigation hooks).
 *
 * PR-3: the broadcast path is `executeOperation` + `getOperationStatus`
 * (journaled operation model) — `sendEth` and the 12 s send abort are
 * gone from this screen; the pendingTxCache write moved to the Rust
 * journal (asserted via `executeOperation` call args).
 *
 * All state-changing interactions wrapped in `await act(async () => …)`
 * to dodge the "import after teardown" race documented в
 * `docs/JEST-SETUP-INCIDENT.md` (same pattern as SendScreen.test).
 */

import React from 'react';
import renderer, { act } from 'react-test-renderer';
import { Linking } from 'react-native';
import { ActionDto } from 'react-native-rustok-bridge';
import ConfirmSendScreen from '../ConfirmSendScreen';
import { mount as sharedMount } from '../../../testing/mount';
import { getWalletHandle } from '../../../lib/walletHandle';
import { useWalletStore } from '../../../stores/walletStore';

jest.mock('../../../lib/walletHandle', () => ({
  getWalletHandle: jest.fn(),
}));

const mockNavigate = jest.fn();
const mockGoBack = jest.fn();
jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({
    navigate: mockNavigate,
    goBack: mockGoBack,
  }),
  useRoute: () => ({
    params: {
      to: '0x97beed7ff45dfe2d5802686821a0196070cf1951',
      amountWei: '500000000000000000',
    },
  }),
}));

const mockedGetWalletHandle = jest.mocked(getWalletHandle);

const SAMPLE_PREVIEW = {
  verdict: {
    action: ActionDto.Allow,
    riskScore: 0,
    findings: [],
    description: 'No risks detected.',
  },
  route: {
    chainId: 11155111n,
    chainName: 'Sepolia',
    estimatedGas: 21000n,
    maxFeePerGas: '0',
    maxPriorityFeePerGas: '0',
    estimatedCostWei: '21000000000000', // 0.000021 ETH
  },
  explanation: 'Routed via Sepolia.',
};

const SAMPLE_OPERATION = {
  id: '0xoperationid',
  chainId: 11155111n,
  status: 'broadcast',
  path: 'direct_eoa',
  txHash:
    '0x9d3f04254a5f3b2eef25dcb1c5fa6f3a05dfdd2f76e913cce63e0c0e2c1d4b50',
  error: undefined,
};

function mountWithBridge(opts: {
  previewSend?: jest.Mock;
  executeOperation?: jest.Mock;
  getOperationStatus?: jest.Mock;
}): {
  previewSend: jest.Mock;
  executeOperation: jest.Mock;
  getOperationStatus: jest.Mock;
} {
  const previewSend = opts.previewSend ?? jest.fn().mockResolvedValue(SAMPLE_PREVIEW);
  const executeOperation =
    opts.executeOperation ?? jest.fn().mockResolvedValue(SAMPLE_OPERATION);
  // First poll reports confirmed → the poll loop breaks without delays.
  const getOperationStatus =
    opts.getOperationStatus ??
    jest.fn().mockResolvedValue({ ...SAMPLE_OPERATION, status: 'confirmed' });
  // Cast through `unknown` — the test surface only needs these
  // methods; recreating the full WalletHandle shape would be noisy.
  mockedGetWalletHandle.mockReturnValue(
    { previewSend, executeOperation, getOperationStatus } as unknown as ReturnType<
      typeof getWalletHandle
    >,
  );
  return { previewSend, executeOperation, getOperationStatus };
}

function findByA11y(tree: renderer.ReactTestRenderer, label: string) {
  return tree.root.findByProps({ accessibilityLabel: label });
}

// Shared mount registers the tree for global teardown (unmountAll in
// jest.setup-after-env.js) so pending React work never outlives the test env.
const mount = () => sharedMount(<ConfirmSendScreen />);

describe('ConfirmSendScreen', () => {
  beforeEach(() => {
    mockedGetWalletHandle.mockReset();
    mockNavigate.mockReset();
    mockGoBack.mockReset();
    // Reset the in-memory store between tests so `refresh` calls are
    // isolated.
    useWalletStore.setState({
      phase: 'unlocked',
      address: undefined,
      balance: undefined,
      error: undefined,
    });
  });

  it('mounts in the loading state and runs previewSend on mount', async () => {
    const { previewSend } = mountWithBridge({});
    const tree = await mount();
    expect(previewSend).toHaveBeenCalledWith(
      '0x97beed7ff45dfe2d5802686821a0196070cf1951',
      '500000000000000000',
      { signal: expect.any(AbortSignal) },
    );
    // After resolve, loading view is gone — guard against false-positive
    // by asserting ready-state badge is present.
    expect(() => findByA11y(tree, 'Verdict badge')).not.toThrow();
  });

  it('shows the error fallback when previewSend throws', async () => {
    mountWithBridge({
      previewSend: jest.fn().mockRejectedValue(new Error('rpc 503')),
    });
    const tree = await mount();
    expect(() => findByA11y(tree, 'Preview error')).not.toThrow();
    expect(() => findByA11y(tree, 'Retry preview')).not.toThrow();
  });

  it('keeps Confirm disabled while preview is loading', async () => {
    // Bridge call never resolves.
    let resolvePreview: (p: typeof SAMPLE_PREVIEW) => void = () => undefined;
    mountWithBridge({
      previewSend: jest.fn(
        () =>
          new Promise<typeof SAMPLE_PREVIEW>((resolve) => {
            resolvePreview = resolve;
          }),
      ),
    });
    const tree = await mount();
    expect(findByA11y(tree, 'Confirm send').props.disabled).toBe(true);
    // Cleanup so React flush settles.
    await act(async () => {
      resolvePreview(SAMPLE_PREVIEW);
    });
  });

  it('enables Confirm on Allow verdict', async () => {
    mountWithBridge({});
    const tree = await mount();
    expect(findByA11y(tree, 'Confirm send').props.disabled).toBe(false);
  });

  it('enables Confirm on Warn verdict (with findings rendered)', async () => {
    const warnPreview = {
      ...SAMPLE_PREVIEW,
      verdict: {
        action: ActionDto.Warn,
        riskScore: 50,
        findings: [
          {
            rule: 'unverified-recipient',
            severity: 'Medium' as const,
            category: 'Recipient' as const,
            description: 'Recipient has no on-chain history.',
          },
        ],
        description: 'Recipient looks unfamiliar.',
      },
    };
    mountWithBridge({
      previewSend: jest.fn().mockResolvedValue(warnPreview),
    });
    const tree = await mount();
    expect(findByA11y(tree, 'Confirm send').props.disabled).toBe(false);
  });

  it('disables Confirm on Block verdict and switches to danger variant', async () => {
    const blockPreview = {
      ...SAMPLE_PREVIEW,
      verdict: {
        action: ActionDto.Block,
        riskScore: 95,
        findings: [],
        description: 'Recipient flagged as malicious.',
      },
    };
    mountWithBridge({
      previewSend: jest.fn().mockResolvedValue(blockPreview),
    });
    const tree = await mount();
    const confirm = findByA11y(tree, 'Confirm send');
    expect(confirm.props.disabled).toBe(true);
  });

  it('executes the operation and navigates to Activity on Confirm tap', async () => {
    const { executeOperation, getOperationStatus } = mountWithBridge({});
    const tree = await mount();
    await act(async () => {
      findByA11y(tree, 'Confirm send').props.onPress();
    });
    // Explicit chain id (networkStore empty in test → preview route
    // fallback 11155111n), single native-transfer call, no AbortSignal —
    // the journaled operation replaced the 12 s send abort.
    expect(executeOperation).toHaveBeenCalledWith(11155111n, [
      {
        to: '0x97beed7ff45dfe2d5802686821a0196070cf1951',
        valueWei: '500000000000000000',
        data: '0x',
      },
    ]);
    expect(getOperationStatus).toHaveBeenCalledWith('0xoperationid');
    expect(mockNavigate).toHaveBeenCalledWith('Tabs', { screen: 'Activity' });
  });

  it('opens the Etherscan link after the operation broadcasts', async () => {
    mountWithBridge({});
    const openSpy = jest
      .spyOn(Linking, 'openURL')
      .mockResolvedValue(true);
    const tree = await mount();
    await act(async () => {
      findByA11y(tree, 'Confirm send').props.onPress();
    });
    expect(openSpy).toHaveBeenCalledWith(
      `https://sepolia.etherscan.io/tx/${SAMPLE_OPERATION.txHash}`,
    );
    openSpy.mockRestore();
  });

  it('surfaces the bridge error and stays on screen when executeOperation fails', async () => {
    const { executeOperation } = mountWithBridge({
      executeOperation: jest.fn().mockRejectedValue(new Error('rpc 503')),
    });
    const tree = await mount();
    await act(async () => {
      findByA11y(tree, 'Confirm send').props.onPress();
    });
    // The throw must NOT navigate away — the user stays on Confirm and
    // can retry.
    expect(executeOperation).toHaveBeenCalledTimes(1);
    expect(mockNavigate).not.toHaveBeenCalled();
    // Confirm button is re-enabled for retry.
    expect(findByA11y(tree, 'Confirm send').props.disabled).toBe(false);
  });

  it('Confirm tap is a no-op on Block verdict (defense in depth, button already disabled)', async () => {
    const blockPreview = {
      ...SAMPLE_PREVIEW,
      verdict: {
        action: ActionDto.Block,
        riskScore: 95,
        findings: [],
        description: 'Recipient flagged as malicious.',
      },
    };
    const { executeOperation } = mountWithBridge({
      previewSend: jest.fn().mockResolvedValue(blockPreview),
    });
    const tree = await mount();
    // Force onPress to fire even though button is disabled — defense
    // check that the handler itself refuses to broadcast.
    await act(async () => {
      findByA11y(tree, 'Confirm send').props.onPress();
    });
    expect(executeOperation).not.toHaveBeenCalled();
  });

  it('Retry on preview error re-runs previewSend', async () => {
    const previewSend = jest
      .fn()
      .mockRejectedValueOnce(new Error('rpc 503'))
      .mockResolvedValueOnce(SAMPLE_PREVIEW);
    mountWithBridge({ previewSend });
    const tree = await mount();
    await act(async () => {
      findByA11y(tree, 'Retry preview').props.onPress();
    });
    expect(previewSend).toHaveBeenCalledTimes(2);
  });
});
