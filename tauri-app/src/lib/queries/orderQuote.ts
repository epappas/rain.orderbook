import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api';
import { rpcUrl } from '$lib/stores/settings';
import type { Order } from '$lib/typeshare/subgraphTypes';
import type { BatchOrderQuotesResponse } from '$lib/typeshare/orderQuote';
import type { Hex } from 'viem';
import { mockIPC } from '@tauri-apps/api/mocks';
import type { RainEvalResultsTable } from '$lib/typeshare/config';

export async function batchOrderQuotes(
  orders: Order[],
  blockNumber?: number,
): Promise<BatchOrderQuotesResponse[]> {
  return invoke('batch_order_quotes', {
    orders,
    blockNumber,
    rpcUrl: get(rpcUrl),
  });
}

export async function debugOrderQuote(
  order: Order,
  inputIOIndex: number,
  outputIOIndex: number,
  orderbook: Hex,
  rpcUrl: string,
) {
  return await invoke<RainEvalResultsTable>('debug_order_quote', {
    order,
    inputIoIndex: inputIOIndex,
    outputIoIndex: outputIOIndex,
    orderbook,
    rpcUrl,
  });
}

export const mockQuoteDebug: RainEvalResultsTable = {
  column_names: ['1', '2', '3'],
  rows: [['0x01', '0x02', '0x03']],
};

if (import.meta.vitest) {
  const { it, expect } = import.meta.vitest;

  it('uses the trade_debug command correctly', async () => {
    mockIPC((cmd) => {
      if (cmd === 'debug_order_quote') {
        return mockQuoteDebug;
      }
    });

    const result = await debugOrderQuote(
      {
        id: '1',
        orderbook: { id: '0x00' },
        orderBytes: '0x123',
        orderHash: '0x123',
        owner: '0x123',
        outputs: [],
        inputs: [],
        active: true,
        addEvents: [],
        timestampAdded: '123',
      },
      0,
      0,
      '0x123',
      'https://rpc-url.com',
    );
    expect(result).toEqual(mockQuoteDebug);
  });
}
