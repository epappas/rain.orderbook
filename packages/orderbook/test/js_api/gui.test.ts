import assert from "assert";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { DotrainOrderGui } from "../../dist/cjs/js_api.js";
import {
  ApprovalCalldata,
  Gui,
  TokenAllowance,
} from "../../dist/types/js_api.js";
import { getLocal } from "mockttp";

const guiConfig = `
gui:
  name: Fixed limit
  description: Fixed limit order strategy
  deployments:
    - deployment: some-deployment
      name: Buy WETH with USDC on Base.
      description: Buy WETH with USDC for fixed price on Base network.
      deposits:
        - token: token1
          min: 0
          presets:
            - "0"
            - "10"
            - "100"
            - "1000"
            - "10000"
      fields:
        - binding: binding-1
          name: Field 1 name
          description: Field 1 description
          presets:
            - name: Preset 1
              value: "0x1234567890abcdef1234567890abcdef12345678"
            - name: Preset 2
              value: "false"
            - name: Preset 3
              value: "some-string"
        - binding: binding-2
          name: Field 2 name
          description: Field 2 description
          min: 100
          presets:
            - value: "99.2"
            - value: "582.1"
            - value: "648.239"
`;
const guiConfig2 = `
gui:
  name: Test test
  description: Test test test
  deployments:
    - deployment: other-deployment
      name: Test test
      description: Test test test
      deposits:
        - token: token1
          min: 0
          presets:
            - "0"
        - token: token2
          min: 0
          presets:
            - "0"
      fields:
        - binding: test-binding
          name: Test binding
          description: Test binding description
          presets:
            - value: "test-value"
`;

const dotrain = `
networks:
    some-network:
        rpc: http://localhost:8085/rpc-url
        chain-id: 123
        network-id: 123
        currency: ETH

subgraphs:
    some-sg: https://www.some-sg.com

deployers:
    some-deployer:
        network: some-network
        address: 0xF14E09601A47552De6aBd3A0B165607FaFd2B5Ba

orderbooks:
    some-orderbook:
        address: 0xc95A5f8eFe14d7a20BD2E5BAFEC4E71f8Ce0B9A6
        network: some-network
        subgraph: some-sg

tokens:
    token1:
        network: some-network
        address: 0xc2132d05d31c914a87c6611c10748aeb04b58e8f
        decimals: 6
        label: T1
        symbol: T1
    token2:
        network: some-network
        address: 0x8f3cf7ad23cd3cadbd9735aff958023239c6a063
        decimals: 18
        label: T2
        symbol: T2

scenarios:
    some-scenario:
        network: some-network
        deployer: some-deployer

orders:
    some-order:
      inputs:
        - token: token1
          vault-id: 1
      outputs:
        - token: token2
          vault-id: 1
      deployer: some-deployer
      orderbook: some-orderbook

deployments:
    some-deployment:
        scenario: some-scenario
        order: some-order
    other-deployment:
        scenario: some-scenario
        order: some-order
---
#calculate-io
_ _: 0 0;
#handle-io
:;
#handle-add-order
:;
`;
const dotrainWithGui = `
${guiConfig}

${dotrain}
`;

describe("Rain Orderbook JS API Package Bindgen Tests - Gui", async function () {
  it("should return error if gui config is not found", async () => {
    await expect(
      DotrainOrderGui.init(dotrain, "some-deployment")
    ).rejects.toEqual(new Error("Gui config not found"));
  });

  it("should initialize gui object", async () => {
    const gui = await DotrainOrderGui.init(dotrainWithGui, "some-deployment");
    const guiConfig = gui.getGuiConfig() as Gui;
    assert.equal(guiConfig.name, "Fixed limit");
    assert.equal(guiConfig.description, "Fixed limit order strategy");
  });

  describe("deposit tests", async () => {
    let gui: DotrainOrderGui;
    beforeAll(async () => {
      gui = await DotrainOrderGui.init(dotrainWithGui, "some-deployment");
    });

    it("should add deposit", async () => {
      gui.saveDeposit("token1", "50.6");
      const deposits = gui.getDeposits();
      assert.equal(deposits.length, 1);
    });

    it("should throw error if deposit token is not found in gui config", () => {
      expect(() => gui.saveDeposit("token3", "1")).toThrow(
        "Deposit token not found in gui config: token3"
      );
    });

    it("should remove deposit", async () => {
      gui.saveDeposit("token1", "50.6");
      const deposits = gui.getDeposits();
      assert.equal(deposits.length, 1);

      gui.removeDeposit("token1");
      const depositsAfterRemove = gui.getDeposits();
      assert.equal(depositsAfterRemove.length, 0);
    });
  });

  describe("field value tests", async () => {
    let gui: DotrainOrderGui;
    beforeAll(async () => {
      gui = await DotrainOrderGui.init(dotrainWithGui, "some-deployment");
    });

    it("should save field value", async () => {
      gui.saveFieldValues([
        {
          binding: "binding-1",
          value: "0x1234567890abcdef1234567890abcdef12345678",
        },
        {
          binding: "binding-2",
          value: "100",
        },
      ]);
      gui.saveFieldValues([
        {
          binding: "binding-1",
          value: "some-string",
        },
        {
          binding: "binding-2",
          value: "true",
        },
      ]);
      const fieldValues = gui.getAllFieldValues();
      assert.equal(fieldValues.length, 2);
    });

    it("should throw error during save if field binding is not found in field definitions", () => {
      expect(() => gui.saveFieldValue("binding-3", "1")).toThrow(
        "Field binding not found: binding-3"
      );
    });

    it("should get field value", async () => {
      gui.saveFieldValue(
        "binding-1",
        "0x1234567890abcdef1234567890abcdef12345678"
      );
      let fieldValue = gui.getFieldValue("binding-1");
      assert.equal(fieldValue, "0x1234567890abcdef1234567890abcdef12345678");

      gui.saveFieldValue("binding-2", "true");
      fieldValue = gui.getFieldValue("binding-2");
      assert.equal(fieldValue, "true");

      gui.saveFieldValue("binding-1", "some-string");
      fieldValue = gui.getFieldValue("binding-1");
      assert.equal(fieldValue, "some-string");

      gui.saveFieldValue("binding-2", "100.5");
      fieldValue = gui.getFieldValue("binding-2");
      assert.equal(fieldValue, "100.5");
    });

    it("should throw error during get if field binding is not found", () => {
      expect(() => gui.getFieldValue("binding-3")).toThrow(
        "Field binding not found: binding-3"
      );
    });
  });

  describe("field definition tests", async () => {
    let gui: DotrainOrderGui;
    beforeAll(async () => {
      gui = await DotrainOrderGui.init(dotrainWithGui, "some-deployment");
    });

    it("should get field definition", async () => {
      const allFieldDefinitions = gui.getAllFieldDefinitions();
      assert.equal(allFieldDefinitions.length, 2);

      const fieldDefinition = gui.getFieldDefinition("binding-1");
      assert.equal(fieldDefinition.name, "Field 1 name");
      assert.equal(fieldDefinition.description, "Field 1 description");
      assert.equal(fieldDefinition.presets.length, 3);

      const preset1 = fieldDefinition.presets[0];
      assert.equal(preset1.name, "Preset 1");
      assert.equal(preset1.value, "0x1234567890abcdef1234567890abcdef12345678");
      const preset2 = fieldDefinition.presets[1];
      assert.equal(preset2.name, "Preset 2");
      assert.equal(preset2.value, "false");
      const preset3 = fieldDefinition.presets[2];
      assert.equal(preset3.name, "Preset 3");
      assert.equal(preset3.value, "some-string");

      const fieldDefinition2 = gui.getFieldDefinition("binding-2");
      assert.equal(fieldDefinition2.presets[0].value, "99.2");
      assert.equal(fieldDefinition2.presets[1].value, "582.1");
      assert.equal(fieldDefinition2.presets[2].value, "648.239");
    });

    it("should throw error during get if field binding is not found", () => {
      expect(() => gui.getFieldDefinition("binding-3")).toThrow(
        "Field binding not found: binding-3"
      );
    });
  });

  describe("state management tests", async () => {
    let serializedString =
      "H4sIAAAAAAAA_3WMPQrCQBCF_YmCnaBlTiBEZjfZ2dnO2lvsZmclCLFJ4Q0EC8XDeAELL-AxvISCUwl5zfuB920GPzFQGYBrh8Aaoq8smWQw-aStJWUiBU-cnEYXuSQyUIEK-D0lE7CGkXBm4qFpY9PuCrWSAY5Kl5VBSw58qCOnvv6P0GMZFMBQ4lS8O-y5VZk0A2tcSn4siskrv21PT5_Pu_M7u1-uH-izWwjtAAAA";
    let gui: DotrainOrderGui;
    beforeAll(async () => {
      gui = await DotrainOrderGui.init(dotrainWithGui, "some-deployment");

      gui.saveFieldValue(
        "binding-1",
        "0x1234567890abcdef1234567890abcdef12345678"
      );
      gui.saveFieldValue("binding-2", "100");
      gui.saveDeposit("token1", "50.6");
    });

    it("should serialize gui state", async () => {
      const serialized = gui.serializeState();
      assert.equal(serialized, serializedString);
    });

    it("should deserialize gui state", async () => {
      gui.clearState();
      gui.deserializeState(serializedString);
      const fieldValues = gui.getAllFieldValues();
      assert.equal(fieldValues.length, 2);
      assert.equal(fieldValues[0].binding, "binding-1");
      assert.equal(
        fieldValues[0].value,
        "0x1234567890abcdef1234567890abcdef12345678"
      );
      assert.equal(fieldValues[1].binding, "binding-2");
      assert.equal(fieldValues[1].value, "100");
      const deposits = gui.getDeposits();
      assert.equal(deposits.length, 1);
      assert.equal(deposits[0].token, "token1");
      assert.equal(deposits[0].amount, "50.6");
      assert.equal(
        deposits[0].address,
        "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
      );
    });

    it("should throw error during deserialize if config is different", async () => {
      let dotrain2 = `
${guiConfig2}

${dotrain}
`;
      let gui2 = await DotrainOrderGui.init(dotrain2, "other-deployment");
      let serialized = gui2.serializeState();
      expect(() => gui.deserializeState(serialized)).toThrow(
        "Deserialized config mismatch"
      );
    });

    it("should clear state", async () => {
      gui.clearState();
      const fieldValues = gui.getAllFieldValues();
      assert.equal(fieldValues.length, 0);
      const deposits = gui.getDeposits();
      assert.equal(deposits.length, 0);
    });
  });

  describe("order operations tests", async () => {
    const mockServer = getLocal();
    let gui: DotrainOrderGui;

    beforeAll(async () => {
      await mockServer.start(8085);
      mockServer.enableDebug();
    });
    afterAll(async () => {
      await mockServer.stop();
    });
    beforeEach(async () => {
      mockServer.reset();

      let dotrain2 = `
      ${guiConfig2}
      
      ${dotrain}
      `;
      gui = await DotrainOrderGui.init(dotrain2, "other-deployment");
    });

    it("checks input and output allowances", async () => {
      // token1 allowance
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0xc2132d05d31c914a87c6611c10748aeb04b58e8f")
        .thenSendJsonRpcResult(
          "0x00000000000000000000000000000000000000000000000000000000000003e8"
        );
      // token2 allowance
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0x8f3cf7ad23cd3cadbd9735aff958023239c6a063")
        .thenSendJsonRpcResult(
          "0x0000000000000000000000000000000000000000000000000000000000000001"
        );

      gui.saveDeposit("token1", "100");
      gui.saveDeposit("token2", "200");

      const allowances: TokenAllowance[] = await gui.checkAllowances(
        "0x1234567890abcdef1234567890abcdef12345678"
      );
      assert.equal(allowances.length, 2);
      assert.equal(
        allowances[0].token,
        "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
      );
      assert.equal(allowances[0].allowance, "0x3e8");
      assert.equal(
        allowances[1].token,
        "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063"
      );
      assert.equal(allowances[1].allowance, "0x1");
    });

    it("generates approval calldatas", async () => {
      // token allowances - 1000
      await mockServer
        .forPost("/rpc-url")
        .thenSendJsonRpcResult(
          "0x00000000000000000000000000000000000000000000000000000000000003e8"
        );

      gui.saveDeposit("token1", "2000");
      gui.saveDeposit("token2", "5000");

      const approvalCalldatas: ApprovalCalldata[] =
        await gui.generateApprovalCalldatas(
          "0x1234567890abcdef1234567890abcdef12345678"
        );
      assert.equal(approvalCalldatas.length, 2);
      assert.equal(
        approvalCalldatas[0].token,
        "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
      );
      // 2000 * 10^6 - 0x3e8 (1000)
      assert.equal(
        approvalCalldatas[0].calldata,
        "0x095ea7b3000000000000000000000000c95a5f8efe14d7a20bd2e5bafec4e71f8ce0b9a60000000000000000000000000000000000000000000000000000000077359018"
      );
      assert.equal(
        approvalCalldatas[1].token,
        "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063"
      );
      // 5000 * 10^18 - 0x3e8 (1000)
      assert.equal(
        approvalCalldatas[1].calldata,
        "0x095ea7b3000000000000000000000000c95a5f8efe14d7a20bd2e5bafec4e71f8ce0b9a600000000000000000000000000000000000000000000010f0cf064dd591ffc18"
      );
    });

    it("generates deposit calldatas", async () => {
      gui.saveDeposit("token1", "2000");
      gui.saveDeposit("token2", "5000");

      const depositCalldatas: string[] = await gui.generateDepositCalldatas();
      assert.equal(depositCalldatas.length, 2);
      assert.equal(
        depositCalldatas[0],
        "0x91337c0a000000000000000000000000c2132d05d31c914a87c6611c10748aeb04b58e8f0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000007735940000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000000"
      );
      assert.equal(
        depositCalldatas[1],
        "0x91337c0a0000000000000000000000008f3cf7ad23cd3cadbd9735aff958023239c6a063000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000010f0cf064dd5920000000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000000"
      );
    });

    it("generates add order calldata", async () => {
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0xf0cfdd37")
        .thenSendJsonRpcResult(`0x${"0".repeat(24) + "1".repeat(40)}`);
      // iStore() call
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0xc19423bc")
        .thenSendJsonRpcResult(`0x${"0".repeat(24) + "2".repeat(40)}`);
      // iParser() call
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0x24376855")
        .thenSendJsonRpcResult(`0x${"0".repeat(24) + "3".repeat(40)}`);
      // parse2() call
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0xa3869e14")
        // 0x1234 encoded bytes
        .thenSendJsonRpcResult(
          "0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000021234000000000000000000000000000000000000000000000000000000000000"
        );

      const addOrderCalldata: string = await gui.generateAddOrderCalldata();
      assert.equal(
        addOrderCalldata,
        "0xa616864d0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000034000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000001e00e9ae0a63bb4a8cf607f57a6e8e0592dd10641c1dd298babe3bec1192f38049cfb5d0f5d713f00f1cfa0edb5ec5acca05ae58ac91e3e589e9ad1bc0082c4383f0000000000000000000000000000000000000000000000000000000000000260000000000000000000000000111111111111111111111111111111111111111100000000000000000000000022222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000212340000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000c2132d05d31c914a87c6611c10748aeb04b58e8f0000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000010000000000000000000000008f3cf7ad23cd3cadbd9735aff958023239c6a063000000000000000000000000000000000000000000000000000000000000001200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000069ff0a89c674ee7874a30058382f2a20302e2063616c63756c6174652d696f202a2f200a5f205f3a203020303b0a0a2f2a20312e2068616e646c652d696f202a2f200a3a3b011bff13109e41336ff20278186170706c69636174696f6e2f6f637465742d73747265616d000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000111111111111111111111111111111111111111100000000000000000000000022222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000212340000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      );
    });

    it("should generate multicalldata for deposit and add order", async () => {
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0xf0cfdd37")
        .thenSendJsonRpcResult(`0x${"0".repeat(24) + "1".repeat(40)}`);
      // iStore() call
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0xc19423bc")
        .thenSendJsonRpcResult(`0x${"0".repeat(24) + "2".repeat(40)}`);
      // iParser() call
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0x24376855")
        .thenSendJsonRpcResult(`0x${"0".repeat(24) + "3".repeat(40)}`);
      // parse2() call
      await mockServer
        .forPost("/rpc-url")
        .withBodyIncluding("0xa3869e14")
        // 0x1234 encoded bytes
        .thenSendJsonRpcResult(
          "0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000021234000000000000000000000000000000000000000000000000000000000000"
        );

      gui.saveDeposit("token1", "2000");
      gui.saveDeposit("token2", "5000");
      const calldata: string = await gui.generateDepositAndAddOrderCalldatas();
      assert.equal(
        calldata,
        "0x82ad56cb00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000002e0000000000000000000000000c95a5f8efe14d7a20bd2e5bafec4e71f8ce0b9a60000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a491337c0a000000000000000000000000c2132d05d31c914a87c6611c10748aeb04b58e8f000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000773594000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c95a5f8efe14d7a20bd2e5bafec4e71f8ce0b9a60000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a491337c0a0000000000000000000000008f3cf7ad23cd3cadbd9735aff958023239c6a063000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000010f0cf064dd592000000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c95a5f8efe14d7a20bd2e5bafec4e71f8ce0b9a6000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000484a616864d0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000034000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000001e0071676168cb34a539137cb8b7bf1a96dcb4e10e3d5d431fc35fca444ffb2c1a0ccf38e58ea9c0a4e3aa90ec4269f6752c0f903abef29b2210ccf4f4f3e1606c20000000000000000000000000000000000000000000000000000000000000260000000000000000000000000111111111111111111111111111111111111111100000000000000000000000022222222222222222222222222222222222222220000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000212340000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000c2132d05d31c914a87c6611c10748aeb04b58e8f0000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000010000000000000000000000008f3cf7ad23cd3cadbd9735aff958023239c6a063000000000000000000000000000000000000000000000000000000000000001200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000069ff0a89c674ee7874a30058382f2a20302e2063616c63756c6174652d696f202a2f200a5f205f3a203020303b0a0a2f2a20312e2068616e646c652d696f202a2f200a3a3b011bff13109e41336ff20278186170706c69636174696f6e2f6f637465742d73747265616d000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000e000000000000000000000000011111111111111111111111111111111111111110000000000000000000000002222222222222222222222222222222222222222000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000021234000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
      );
    });
  });
});
