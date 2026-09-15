// ==========================================
// Rust(Wasm)の機能を読み込む設定
// ==========================================
import init, {
  process_audio_wasm,
  get_file_metadata_wasm,
} from "./wsd_logic/pkg/wsd_logic.js";

document.addEventListener("DOMContentLoaded", async () => {
  try {
    await init(); // Rustのプログラムをロード
    console.log("Wasm (Rust) initialized successfully!");
  } catch (e) {
    console.error("Wasm initialization failed:", e);
    alert(
      "Wasmの読み込みに失敗しました。`wasm-pack build --target web` を実行しましたか？"
    );
  }

  // ==========================================
  // 2. UI要素の取得 (既存のまま)
  // ==========================================
  const audioFileInput = document.getElementById("audioFileInput");
  const fileSelectBtn = document.getElementById("fileSelectBtn");
  const processButton = document.getElementById("processButton");
  const statusMessage = document.getElementById("statusMessage");
  const downloadLink = document.getElementById("downloadLink");
  const dropArea = document.getElementById("dropArea");
  const dropContent = document.getElementById("dropContent");
  const fileListContent = document.getElementById("fileListContent");
  const fileListElement = document.getElementById("fileList");
  const addMoreBtn = document.getElementById("addMoreBtn");
  const tabs = document.querySelectorAll(".tab");
  const tabSlider = document.querySelector(".tab-slider");

  // ナビゲーションボタン
  const backBtnMeta = document.getElementById("backBtnMeta");
  const homeBtnMeta = document.getElementById("homeBtnMeta");
  const backBtnAudio = document.getElementById("backBtnAudio");
  const homeBtnAudio = document.getElementById("homeBtnAudio");

  // ヘルプモーダル関連
  const helpModal = document.getElementById("helpModal");
  const modalTitle = document.getElementById("modalTitle");
  const modalText = document.getElementById("modalText");
  const closeModal = document.getElementById("closeModal");
  const helpIcons = document.querySelectorAll(".help-icon");

  // メタデータ画面関連
  const mainScreen = document.getElementById("mainScreen");
  const metadataScreen = document.getElementById("metadataScreen");
  const skipMetadata = document.getElementById("skipMetadata");
  const metadataInputs = document.querySelectorAll(".metadata-input");
  const processWithMetadata = document.getElementById("processWithMetadata");
  const metadataStatusMessage = document.getElementById(
    "metadataStatusMessage"
  );
  const metadataDownloadLink = document.getElementById("metadataDownloadLink");

  // 音声パラメータ画面関連
  const audioParamsScreen = document.getElementById("audioParamsScreen");
  const processWithAudioParams = document.getElementById(
    "processWithAudioParams"
  );
  const audioParamsStatusMessage = document.getElementById(
    "audioParamsStatusMessage"
  );
  const audioParamsDownloadLink = document.getElementById(
    "audioParamsDownloadLink"
  );
  const channelConfigListElement = document.getElementById("channelConfigList"); // 新規追加

  // ファイル形式別設定
  const binSettings = document.getElementById("binSettings");
  const audioSettings = document.getElementById("audioSettings");
  const currentSampleRateInput = document.getElementById("currentSampleRate");

  // bin設定用要素
  const samplingFreqSelect = document.getElementById("samplingFrequency");
  const customSamplingFreqInput = document.getElementById(
    "customSamplingFrequency"
  );
  const channelCountDisplay = document.getElementById("channelCountDisplay");
  const channelCountHidden = document.getElementById("channelCount");

  let fileList = [];
  let selectedFile = null;
  let currentTab = "bin";

  // ファイルごとのチャンネル割り当て管理
  const fileChannelMap = new Map();

  const CHANNEL_PRESETS = {
    1: ["cf"],
    2: ["lf", "rf"],
    3: ["lf", "cf", "rf"],
    4: ["lf", "rf", "lr", "rr"],
    5: ["lf", "rf", "cf", "lr", "rr"],
    6: ["lf", "rf", "cf", "lfe", "lr", "rr"],
  };

  // 全チャンネルリスト（選択肢用）
  const ALL_CHANNELS = [
    { value: "lf", label: "Lf" },
    { value: "lf-middle", label: "Lf-m" },
    { value: "cf", label: "Cf" },
    { value: "rf-middle", label: "Rf-m" },
    { value: "rf", label: "Rf" },
    { value: "lr", label: "Lr" },
    { value: "lr-middle", label: "Lr-m" },
    { value: "cr", label: "Cr" },
    { value: "rr-middle", label: "Rr-m" },
    { value: "rr", label: "Rr" },
    { value: "lfe", label: "LFE" },
  ];

  // ヘルプデータ
  const helpData = {
    main: {
      title: "本アプリについて",
      text: "ブラウザ完結型のWSDフォーマット変換ツールです。\n\nRust + WebAssembly を使用し、サーバーへのアップロードを行わずにローカル環境で変換処理を実行します。",
    },
    bin: {
      title: "binファイルについて",
      text: "ヘッダー無しの生データをWSDに変換するモードです。\n\nチャンネルごとに分割されたモノラルファイルを複数選択し、WSD仕様に基づいて変換します。\n※標本化周波数・チャンネル配置の手動設定が必要です。",
    },
    "audio-high": {
      title: "DSD音源について",
      text: "DSF / DSDIFFをWSDに変換するモードです。\n\nファイルのヘッダーを解析し、1bitデータ部分のみを抽出してWSD化します。\nメタデータは自動取得されるため、手動設定は不要です。",
    },
    "audio-common": {
      title: "PCM音源について",
      text: "未対応",
    },
  };

  // ウィンドウサイズ変更処理
  window.addEventListener("resize", () => {
    setTimeout(() => window.scrollTo(0, 0), 100);
  });
  window.addEventListener("orientationchange", () => {
    setTimeout(() => window.scrollTo(0, 0), 500);
  });
  if ("ontouchstart" in window)
    document.body.style.webkitOverflowScrolling = "touch";

  // ヘルプモーダル表示関数
  function showHelpModal(helpType) {
    const data = helpData[helpType];
    if (data) {
      requestAnimationFrame(() => {
        modalTitle.textContent = data.title;
        modalText.textContent = data.text;
        helpModal.style.display = "block";
        document.body.style.overflow = "hidden";
      });
    }
  }
  function hideHelpModal() {
    requestAnimationFrame(() => {
      helpModal.style.display = "none";
      document.body.style.overflow = "auto";
    });
  }

  // ヘルプアイコンイベント
  helpIcons.forEach((icon) => {
    icon.addEventListener(
      "click",
      (e) => {
        e.stopPropagation();
        e.preventDefault();
        const helpType = icon.dataset.help;
        requestAnimationFrame(() => showHelpModal(helpType));
      },
      { passive: false }
    );
  });
  closeModal.addEventListener("click", hideHelpModal);
  helpModal.addEventListener("click", (e) => {
    if (e.target === helpModal) hideHelpModal();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") hideHelpModal();
  });

  // タブスライダー制御
  let updateSliderTimeout;
  function updateTabSlider() {
    clearTimeout(updateSliderTimeout);
    updateSliderTimeout = setTimeout(() => {
      const activeTab = document.querySelector(".tab.active");
      if (!activeTab) return;
      const tabIndex = Array.from(tabs).indexOf(activeTab);
      const sliderWidth = 100 / tabs.length;
      const translateX = tabIndex * 100;
      requestAnimationFrame(() => {
        tabSlider.style.width = `${sliderWidth}%`;
        tabSlider.style.transform = `translateX(${translateX}%)`;
      });
    }, 16);
  }
  updateTabSlider();

  // タブ切り替え
  tabs.forEach((tab) => {
    tab.addEventListener(
      "click",
      (e) => {
        if (
          e.target.classList.contains("help-icon") ||
          e.target.classList.contains("tab-help") ||
          tab.classList.contains("active") ||
          tab.classList.contains("disabled")
        )
          return;
        requestAnimationFrame(() => {
          tabs.forEach((t) => t.classList.remove("active"));
          tab.classList.add("active");
          currentTab = tab.dataset.tab;
          updateTabSlider();
          updateFileAccept();
          resetFileSelection();

          if (currentTab === "audio-high") {
            processButton.textContent = "処理を開始";
          } else {
            processButton.textContent = "次に進む";
          }
        });
      },
      { passive: true }
    );
  });

  function updateFileAccept() {
    const acceptMap = {
      bin: ".bin",
      "audio-high": ".dsf, .dff",
      "audio-common": ".wav, .aif",
    };
    audioFileInput.accept =
      acceptMap[currentTab] || ".bin, .dsf, .dff, .aif, .wav";

    // binタブの場合のみ複数選択を許可
    if (currentTab === "bin") {
      audioFileInput.multiple = true;
    } else {
      audioFileInput.multiple = false;
    }

    // タブ切り替え時にファイルリストをリセットするかどうか
    if (fileList.length > 0) {
      const allValid = fileList.every(isFileTypeAllowed);
      if (!allValid) {
        resetFileSelection();
      } else if (currentTab !== "bin" && fileList.length > 1) {
        // bin以外で複数選択されている場合は最初の1つだけ残す
        fileList = [fileList[0]];
        renderFileList();
      }
    }
  }

  function resetFileSelection() {
    fileList = [];
    fileChannelMap.clear();
    selectedFile = null;
    audioFileInput.value = "";
    renderFileList(); // UI更新
    processButton.disabled = true;
    statusMessage.textContent = "";
    downloadLink.style.display = "none";

    // 表示切り替え
    dropContent.style.display = "flex";
    fileListContent.style.display = "none";
  }

  fileSelectBtn.addEventListener("click", () => audioFileInput.click());
  addMoreBtn.addEventListener("click", () => audioFileInput.click());

  // ドラッグ&ドロップ
  dropArea.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropArea.classList.add("dragover");
  });
  dropArea.addEventListener("dragleave", (e) => {
    e.preventDefault();
    dropArea.classList.remove("dragover");
  });
  dropArea.addEventListener("drop", (e) => {
    e.preventDefault();
    dropArea.classList.remove("dragover");
    const droppedFiles = Array.from(e.dataTransfer.files);

    if (droppedFiles.length > 0) {
      handleFiles(droppedFiles);
    }
  });

  function isFileTypeAllowed(file) {
    const fileName = file.name.toLowerCase();
    const allowedExtensions = {
      bin: [".bin"],
      "audio-high": [".dsf", ".dff"],
      "audio-common": [".wav, .aif"],
    };
    const extensions = allowedExtensions[currentTab] || [];
    return extensions.some((ext) => fileName.endsWith(ext.replace(" ", "")));
  }

  audioFileInput.addEventListener("change", (event) => {
    if (event.target.files && event.target.files.length > 0) {
      handleFiles(Array.from(event.target.files));
    }
    // 同じファイルを選べるようにリセット
    audioFileInput.value = "";
  });

  function handleFiles(newFiles) {
    // 対応していないファイルを除外
    const validFiles = newFiles.filter(isFileTypeAllowed);
    if (validFiles.length === 0) {
      statusMessage.textContent =
        "選択されたタブに対応していないファイル形式です。";
      return;
    }

    if (currentTab === "bin") {
      // binモード: 追加モード
      fileList.push(...validFiles);
    } else {
      // 他のモード: 置換モード（1つだけ）
      fileList = [validFiles[0]];
      fileChannelMap.clear();
    }

    // チャンネル自動割り当て
    reassignChannels();

    renderFileList();
  }

  function reassignChannels() {
    if (currentTab !== "bin") return;

    const count = fileList.length;
    const preset = CHANNEL_PRESETS[count] || [];

    fileChannelMap.clear();

    fileList.forEach((file, index) => {
      let ch = "lf";
      if (index < preset.length) {
        ch = preset[index];
      } else if (index < ALL_CHANNELS.length) {
        ch = ALL_CHANNELS[index].value;
      }
      fileChannelMap.set(file, ch);
    });

    updateCheckboxDisplay();
  }

  // 変換結果を .wsd としてダウンロードさせる。
  // ObjectURL は使い終わったら必ず解放する（未解放だとタブを閉じるまでメモリが残る）
  function downloadResult(resultBytes, linkElement, sourceFileName) {
    const blob = new Blob([resultBytes], {
      type: "application/octet-stream",
    });
    const url = URL.createObjectURL(blob);
    const baseName = sourceFileName.replace(/\.[^/.]+$/, "");

    linkElement.href = url;
    linkElement.download = `processed_${baseName}.wsd`;
    linkElement.click();

    // click() がダウンロードを開始したあとに解放する
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  // メタデータ入力欄を集めて Rust 側のフィールド名に変換する
  function collectMetadata() {
    const metadata = {};

    // HTMLのID -> Rustのフィールド名(camelCase) への変換マップ
    const idMap = {
      title: "title",
      composer: "composer",
      lyricist: "songWriter",
      artist: "artist",
      album: "album",
      genre: "genre",
      recordDate: "dateTime",
      recordLocation: "location",
      comment: "comment",
      other: "userSpecific",
    };

    if (!skipMetadata.checked) {
      metadataInputs.forEach((input) => {
        // disabledでない、かつ値が入っているものだけ送る
        if (!input.disabled && input.value.trim()) {
          const rustKey = idMap[input.id] || input.id;
          metadata[rustKey] = input.value.trim();
        }
      });
    }

    return metadata;
  }

  function formatFileSize(bytes) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  }

  function renderFileList() {
    fileListElement.innerHTML = "";

    if (fileList.length === 0) {
      statusMessage.textContent = "ファイルが選択されていません。";
      processButton.disabled = true;
      downloadLink.style.display = "none";
      selectedFile = null;

      // ファイルがない場合はドロップエリアを表示
      dropContent.style.display = "flex";
      fileListContent.style.display = "none";
      return;
    }

    // ファイルがある場合はリストを表示
    dropContent.style.display = "none";
    fileListContent.style.display = "flex";

    // 単一ファイルモードかどうかでクラスを切り替え
    if (currentTab !== "bin") {
      fileListElement.classList.add("single-mode");
    } else {
      fileListElement.classList.remove("single-mode");
    }

    // ファイルリストUI構築
    fileList.forEach((file, index) => {
      const item = document.createElement("div");
      item.className = "file-item";

      // アイコン追加
      const icon = document.createElement("div");
      icon.className = "file-item-icon";
      icon.textContent = "🎵";

      const info = document.createElement("div");
      info.className = "file-info";

      const name = document.createElement("span");
      name.className = "file-name";
      name.textContent =
        currentTab === "bin" ? `${index + 1}. ${file.name}` : file.name;

      const size = document.createElement("span");
      size.className = "file-size";
      size.textContent = `(${formatFileSize(file.size)})`;

      info.appendChild(name);
      info.appendChild(size);

      const removeBtn = document.createElement("div");
      removeBtn.className = "remove-btn";
      removeBtn.innerHTML = "&times;";
      removeBtn.title = "削除";
      removeBtn.onclick = () => {
        fileList.splice(index, 1);
        fileChannelMap.delete(file);
        reassignChannels(); // 削除時も再割り当て
        renderFileList();
      };

      item.appendChild(icon);
      item.appendChild(info);
      item.appendChild(removeBtn);
      fileListElement.appendChild(item);
    });

    // 「+ ファイルを追加」ボタンの表示制御
    // bin以外のタブで、既にファイルが1つある場合は追加ボタンを隠す
    if (currentTab !== "bin" && fileList.length >= 1) {
      addMoreBtn.style.display = "none";
    } else {
      addMoreBtn.style.display = "block";
    }

    // 状態更新
    if (fileList.length === 1) {
      statusMessage.textContent = `ファイルが選択されています (1件)`;
    } else {
      statusMessage.textContent = `${fileList.length}個のファイルが選択されています`;
    }

    processButton.disabled = false;
    downloadLink.style.display = "none";

    // 設定画面の更新は「1つ目のファイル」を基準に行う
    selectedFile = fileList[0];
    updateAudioParamsSettings();
  }

  function updateAudioParamsSettings() {
    if (!selectedFile) return;
    const fileExtension =
      "." + selectedFile.name.split(".").pop().toLowerCase();
    binSettings.style.display = "none";
    audioSettings.style.display = "none";

    if (fileExtension === ".bin") {
      binSettings.style.display = "block";

      // チャンネル数表示の更新
      const count = fileList.length > 0 ? fileList.length : 1;
      channelCountHidden.value = count;

      // チャンネル設定リストの生成
      channelConfigListElement.innerHTML = "";
      fileList.forEach((file, index) => {
        const item = document.createElement("div");
        item.className = "file-item";
        item.style.marginBottom = "0.5vh";
        item.style.padding = "0.5vh";

        const info = document.createElement("div");
        info.className = "file-info";

        const name = document.createElement("span");
        name.className = "file-name";
        name.textContent = `${index + 1}. ${file.name}`;
        name.style.fontSize = "0.9rem";

        info.appendChild(name);

        // チャンネル選択プルダウン
        const select = document.createElement("select");
        select.className = "channel-select";

        ALL_CHANNELS.forEach((ch) => {
          const option = document.createElement("option");
          option.value = ch.value;
          option.textContent = ch.label;
          if (fileChannelMap.get(file) === ch.value) {
            option.selected = true;
          }
          select.appendChild(option);
        });

        select.addEventListener("change", (e) => {
          fileChannelMap.set(file, e.target.value);
          updateCheckboxDisplay(); // チェックボックスも即時更新
        });

        item.appendChild(info);
        item.appendChild(select);
        channelConfigListElement.appendChild(item);
      });

      updateCheckboxDisplay();
    } else if ([".wav", ".aif"].includes(fileExtension)) {
      audioSettings.style.display = "block";
      currentSampleRateInput.value = 44100; // 仮の値
      currentSampleRateInput.readOnly = false;
    }
  }

  // Rustを使ってファイルのメタデータを抽出し、画面に反映する
  async function extractAndFillMetadata(file) {
    if (!file) return;

    const ext = "." + file.name.split(".").pop().toLowerCase();
    if (![".dsf", ".dff"].includes(ext)) return;

    try {
      // 読み込み
      const buffer = new Uint8Array(await file.arrayBuffer());

      const meta = get_file_metadata_wasm(buffer);

      if (meta && meta.hasData) {
        console.log("メタデータ抽出成功:", meta);

        // 画面のフィールドに値をセット
        if (meta.title) document.getElementById("title").value = meta.title;
        if (meta.artist) document.getElementById("artist").value = meta.artist;
        if (meta.album) document.getElementById("album").value = meta.album;
        if (meta.composer)
          document.getElementById("composer").value = meta.composer;
        if (meta.genre) document.getElementById("genre").value = meta.genre;
        if (meta.dateTime)
          document.getElementById("recordDate").value = meta.dateTime;

        // 「情報を入力しない」チェックを外す
        skipMetadata.checked = false;
        metadataInputs.forEach((input) => {
          input.disabled = false;
          input.classList.remove("input-warning");
        });

        statusMessage.textContent = "ファイルからメタデータを読み込みました。";
      }
    } catch (e) {
      console.warn("メタデータ抽出失敗:", e);
      // 失敗しても止まらず、空欄のまま進む
    }
  }

  function updateCheckboxDisplay() {
    const checkboxes = document.querySelectorAll(
      '#binSettings input[type="checkbox"]'
    );
    checkboxes.forEach((cb) => (cb.checked = false)); // 一旦リセット

    fileList.forEach((file) => {
      const assignedCh = fileChannelMap.get(file);
      if (assignedCh) {
        const targetCb = document.querySelector(
          `#binSettings input[value="${assignedCh}"]`
        );
        if (targetCb) targetCb.checked = true;
      }
    });
  }

  // 標本化周波数の手入力切り替え
  if (samplingFreqSelect) {
    samplingFreqSelect.addEventListener("change", (e) => {
      if (e.target.value === "other") {
        customSamplingFreqInput.style.display = "block";
        customSamplingFreqInput.required = true;
      } else {
        customSamplingFreqInput.style.display = "none";
        customSamplingFreqInput.required = false;
      }
    });
  }

  // 画面遷移ロジック
  processButton.addEventListener("click", async () => {
    if (fileList.length === 0) {
      statusMessage.textContent = "ファイルを先に選択してください。";
      return;
    }

    // DSDモードの場合は画面遷移せずに即時実行
    if (currentTab === "audio-high") {
      await processDsdImmediately();
    } else {
      slideToMetadataScreen();
    }
  });

  async function processDsdImmediately() {
    processButton.disabled = true;
    statusMessage.textContent = "処理中です...";
    downloadLink.style.display = "none";

    try {
      const file = fileList[0];
      const buffer = new Uint8Array(await file.arrayBuffer());

      // パラメータ・メタデータは空でRustに任せる（DSDはヘッダーから自動解析される）
      const metadataStr = JSON.stringify({});
      const paramsStr = JSON.stringify({});

      const resultBytes = process_audio_wasm(buffer, metadataStr, paramsStr);

      downloadResult(resultBytes, downloadLink, file.name);

      statusMessage.textContent = "処理完了！";
    } catch (e) {
      console.error(e);
      const errorMsg = typeof e === "string" ? e : e.message;
      statusMessage.textContent = "エラー: " + errorMsg;
    } finally {
      processButton.disabled = false;
    }
  }

  async function slideToMetadataScreen() {
    if (selectedFile) {
      await extractAndFillMetadata(selectedFile);
    }
    window.scrollTo(0, 0);
    requestAnimationFrame(() => {
      mainScreen.classList.add("slide-left");
      setTimeout(() => {
        metadataScreen.classList.add("show");
      }, 150);
      setTimeout(() => {
        window.scrollTo(0, 0);
      }, 300);
    });
  }

  function slideToAudioParamsScreen() {
    window.scrollTo(0, 0);
    updateAudioParamsSettings(); 
    requestAnimationFrame(() => {
      metadataScreen.classList.remove("show");
      setTimeout(() => {
        audioParamsScreen.classList.add("show");
      }, 150);
      setTimeout(() => {
        window.scrollTo(0, 0);
      }, 300);
    });
  }

  function updateAudioParamsDisplay() {
    if (selectedFile) {
      updateAudioParamsSettings();
    }
  }

  skipMetadata.addEventListener("change", () => {
    const isChecked = skipMetadata.checked;
    metadataInputs.forEach((input) => {
      input.disabled = isChecked;
      if (isChecked) {
        input.value = "";
        input.classList.remove("input-warning");
      }
    });
  });

  // 音声パラメータ画面からの処理開始
  processWithAudioParams.addEventListener("click", async () => {
    if (fileList.length === 0) return;

    const firstFile = fileList[0];
    const fileExtension = "." + firstFile.name.split(".").pop().toLowerCase();

    // メッセージ更新
    audioParamsStatusMessage.textContent = "処理中...";

    // パラメータ収集用オブジェクト
    const audioParams = {};

    // Rustへ渡すバイナリデータ
    let dataToProcess = null;

    try {
      if (fileExtension === ".bin") {
        // 1. 設定値の取得
        const freqVal = samplingFreqSelect.value;
        audioParams.samplingFrequency = parseInt(
          freqVal === "other" ? customSamplingFreqInput.value : freqVal,
          10
        );

        const chCount = fileList.length;
        audioParams.channelCount = chCount;

        // 2. 現在のファイルリストとプルダウン設定をマッピング
        const currentAssignment = new Map();

        // ファイルごとに設定されたチャンネルを取得
        let isConfigIncomplete = false;
        fileList.forEach((file) => {
          const assignedCh = fileChannelMap.get(file);
          if (assignedCh) {
            currentAssignment.set(assignedCh, file);
          } else {
            isConfigIncomplete = true;
          }
        });

        if (isConfigIncomplete) {
          audioParamsStatusMessage.textContent =
            "エラー: チャンネル設定がされていないファイルがあります。";
          return;
        }

        // 3. WSD規格上のチャンネル順序定義
        const WSD_ORDER = [
          "lf",
          "lf-middle",
          "cf",
          "rf-middle",
          "rf", 
          "lfe", // LFE (FrontのLSB)
          "lr",
          "lr-middle",
          "cr",
          "rr-middle",
          "rr",
        ];

        // 4. WSD順序に従ってファイルをピックアップ
        const filesToLoad = [];
        const sortedChannelLayout = [];

        for (const ch of WSD_ORDER) {
          if (currentAssignment.has(ch)) {
            filesToLoad.push(currentAssignment.get(ch));
            sortedChannelLayout.push(ch);
          }
        }

        if (filesToLoad.length !== chCount) {
          audioParamsStatusMessage.textContent = `エラー: チャンネル設定に重複があるか、無効な構成です。(ファイル数:${chCount}, 有効設定:${filesToLoad.length})`;
          return;
        }

        // Rustへ渡すパラメータに追加
        audioParams.inputFileCount = filesToLoad.length;
        audioParams.channelLayout = sortedChannelLayout;
        // 5. ファイル読み込み & 結合
        let totalSize = 0;
        for (const file of filesToLoad) {
          totalSize += file.size;
        }

        const mergedBuffer = new Uint8Array(totalSize);
        let offset = 0;

        for (const file of filesToLoad) {
          const buffer = new Uint8Array(await file.arrayBuffer());
          mergedBuffer.set(buffer, offset);
          offset += buffer.length;
        }

        dataToProcess = mergedBuffer;
      } else {
        // 基本的に1つ目のファイルのみ処理する
        dataToProcess = new Uint8Array(await firstFile.arrayBuffer());

        // WAV/MP3の場合のみパラメータが必要
        if ([".wav", ".aif"].includes(fileExtension)) {
          audioParams.samplingMultiplier =
            document.getElementById("samplingMultiplier").value;
          audioParams.currentSampleRate = currentSampleRateInput.value;
        }
      }

      const metadata = collectMetadata();
      const metadataStr = JSON.stringify(metadata);

      const paramsStr = JSON.stringify(audioParams);

      console.log("Rustへ送信:", paramsStr); // デバッグ用

      const resultBytes = process_audio_wasm(
        dataToProcess,
        metadataStr,
        paramsStr
      );

      downloadResult(resultBytes, audioParamsDownloadLink, firstFile.name);

      audioParamsStatusMessage.textContent = "処理完了！";
    } catch (error) {
      console.error("Processing error:", error);
      // Rustから返ってきたエラーメッセージを表示
      const errorMsg = typeof error === "string" ? error : error.message;
      audioParamsStatusMessage.textContent = "エラー: " + errorMsg;
    }
  });

  // メタデータ画面からの処理開始
  processWithMetadata.addEventListener("click", async () => {
    if (!selectedFile) return;

    // 次の画面が必要かチェック
    const fileExtension =
      "." + selectedFile.name.split(".").pop().toLowerCase();
    const needsAudioParams = [".bin", ".wav", ".aif"].includes(fileExtension);
    if (needsAudioParams) {
      slideToAudioParamsScreen();
      return;
    }

    // 直接処理開始
    metadataStatusMessage.textContent = "処理中...";
    processWithMetadata.disabled = true;

    try {
      const metadata = collectMetadata();

      const fileBuffer = new Uint8Array(await selectedFile.arrayBuffer());
      const metadataStr = JSON.stringify(metadata);
      const paramsStr = JSON.stringify({});

      const resultBytes = process_audio_wasm(
        fileBuffer,
        metadataStr,
        paramsStr
      );

      downloadResult(resultBytes, metadataDownloadLink, selectedFile.name);

      metadataStatusMessage.textContent = "Rustによる処理が完了しました！";
    } catch (error) {
      console.error("Processing error:", error);
      metadataStatusMessage.textContent = "エラーが発生しました: " + error;
    } finally {
      processWithMetadata.disabled = false;
    }
  });

  function isValidASCII(text) {
    return /^[\x20-\x7E]*$/.test(text);
  }
  const characterLimits = {
    genre: 32,
    recordLocation: 32,
    comment: 512,
    other: 512,
    title: 128,
    composer: 128,
    lyricist: 128,
    artist: 128,
    album: 128,
    recordDate: 128,
  };

  metadataInputs.forEach((input) => {
    input.addEventListener("input", (e) => {
      const value = e.target.value;
      if (!isValidASCII(value)) {
        e.target.classList.add("input-warning");
      } else {
        e.target.classList.remove("input-warning");
      }
    });
  });

  updateFileAccept();

  // 戻るボタン処理
  if (backBtnMeta) {
    backBtnMeta.addEventListener("click", () => {
      // メタデータ画面 -> メイン画面
      requestAnimationFrame(() => {
        metadataScreen.classList.remove("show");
        setTimeout(() => {
          mainScreen.classList.remove("slide-left");
          window.scrollTo(0, 0);
        }, 300);
      });
    });
  }

  if (backBtnAudio) {
    backBtnAudio.addEventListener("click", () => {
      // パラメータ画面 -> メタデータ画面
      requestAnimationFrame(() => {
        audioParamsScreen.classList.remove("show");
        setTimeout(() => {
          metadataScreen.classList.add("show");
          window.scrollTo(0, 0);
        }, 150);
      });
    });
  }

  // ホームボタン処理（共通）
  function goHome() {
    requestAnimationFrame(() => {
      audioParamsScreen.classList.remove("show");
      metadataScreen.classList.remove("show");

      setTimeout(() => {
        mainScreen.classList.remove("slide-left");
        resetFileSelection(); // データをリセット
        window.scrollTo(0, 0);
      }, 300);
    });
  }

  if (homeBtnMeta) homeBtnMeta.addEventListener("click", goHome);
  if (homeBtnAudio) homeBtnAudio.addEventListener("click", goHome);
});
