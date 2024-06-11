import { Withdraw } from "../generated/OrderBook/OrderBook";
import { Withdrawal } from "../generated/schema";
import { createTransactionEntity } from "./transaction";

export function handleWithdraw(event: Withdraw): void {
  createWithdrawalEntity(event);
}

export function createWithdrawalEntity(event: Withdraw): void {
  let withdraw = new Withdrawal(
    event.transaction.hash.concatI32(event.logIndex.toI32())
  );
  withdraw.amount = event.params.amount;
  withdraw.targetAmount = event.params.targetAmount;
  withdraw.sender = event.params.sender;
  withdraw.vaultId = event.params.vaultId;
  withdraw.token = event.params.token;
  withdraw.transaction = createTransactionEntity(event);
  withdraw.save();
}
