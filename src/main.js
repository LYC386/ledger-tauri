const { invoke } = window.__TAURI__.tauri;


let pkInputEl;
let pkMsgEl;

let dataPkInputEl;
let dataChainIdInputEl;
let dataInputEl;
let dataMsgEl;

let txPkInputEl;
let txChainIdInputEl;
let txToInputEl;
let txValInputEl;
let txNonceInputEl;
let txPriorityFeeInputEl;
let txMaxFeeInputEl;
let txMsgEl;



async function get_pk() {
  invoke("get_pk", { num: pkInputEl.value })
    .then((pk) => pkMsgEl.textContent = pk)
    .catch((error) => pkMsgEl.textContent = error);
}

async function sign_data() {
  dataMsgEl.textContent = "Comfirm on Ledger";
  invoke("sign_data", { num: dataPkInputEl.value, msg: dataInputEl.value, chainId: dataChainIdInputEl.value })
    .then((sig) => dataMsgEl.textContent = sig)
    .catch((error) => dataMsgEl.textContent = error);
}

async function sign_tx() {
  txMsgEl.textContent = "Comfirm on Ledger";
  invoke("sign_tx", { num: txPkInputEl.value, chainId: txChainIdInputEl.value, value: txValInputEl.value, to: txToInputEl.value, nonce: txNonceInputEl.value, priorityFee: txPriorityFeeInputEl.value, maxFee: txMaxFeeInputEl.value })
    .then((sigTx) => txMsgEl.textContent = sigTx)
    .catch((error) => txMsgEl.textContent = error);
}



window.addEventListener("DOMContentLoaded", () => {

  pkInputEl = document.querySelector("#pk-input");
  pkMsgEl = document.querySelector("#pk-msg");
  document
    .querySelector("#pk-button")
    .addEventListener("click", () => get_pk());

  dataPkInputEl = document.querySelector("#sign-data-pk-input");
  dataChainIdInputEl = document.querySelector("#sign-data-chain-input");
  dataInputEl = document.querySelector("#data");
  dataMsgEl = document.querySelector("#sig-data-msg");
  document
    .querySelector("#sign-data-button")
    .addEventListener("click", () => sign_data());

  txPkInputEl = document.querySelector("#tx-pk-input");
  txChainIdInputEl = document.querySelector("#tx-chain-input");
  txToInputEl = document.querySelector("#tx-to-input");
  txValInputEl = document.querySelector("#tx-value-input");
  txNonceInputEl = document.querySelector("#tx-nonce-input");
  txPriorityFeeInputEl = document.querySelector("#tx-priority-fee-input");
  txMaxFeeInputEl = document.querySelector("#tx-max-fee-input");
  txMsgEl = document.querySelector("#sig-tx-msg");
  document
    .querySelector("#sign-tx-button")
    .addEventListener("click", () => sign_tx());
});


