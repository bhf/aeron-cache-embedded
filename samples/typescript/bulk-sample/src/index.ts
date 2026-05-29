import { AeronCacheClient } from 'aeron-cache-embedded-client';

async function main() {
    const args = process.argv.slice(2);
    const baseUrl = args[0] || 'http://localhost:7070';
    const wsUrl = args[1] || 'ws://localhost:7071';

    console.log(`Starting TypeScript Bulk Operations Sample against ${baseUrl}`);

    const client = new AeronCacheClient(baseUrl, wsUrl);
    const cacheId = 'bulk-ts-sample';

    const bulkRequest = {
        requestId: Math.random().toString(36).substring(7),
        operations: [
            {
                operationType: 'CREATE_CACHE' as const,
                requestId: 'op-1',
                cacheId: cacheId
            },
            {
                operationType: 'ADD_ITEM' as const,
                requestId: 'op-2',
                cacheId: cacheId,
                key: 'ts-bulk-1',
                value: 'value-1'
            },
            {
                operationType: 'GET_ITEM' as const,
                requestId: 'op-3',
                cacheId: cacheId,
                key: 'ts-bulk-1'
            }
        ]
    };

    try {
        console.log('Executing bulk operations...');
        const response = await client.bulkOps(bulkRequest);

        console.log(`Bulk Response ID: ${response.requestId}`);
        for (const opResp of response.operationResponses) {
            console.log(`  Op ${opResp.requestId}: status=${opResp.status}, cache=${opResp.cacheId}, key=${opResp.key}, value=${opResp.value}`);
        }
    } catch (error) {
        console.error('Error executing bulk operations:', error);
    }
}

main();
