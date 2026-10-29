use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::str;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// cargo test (ネイティブ実行) では console が無いので何もしない
#[cfg(not(target_arch = "wasm32"))]
fn log(_s: &str) {}

// メタデータ 
#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")] 
struct Metadata {
    title: Option<String>,
    composer: Option<String>,
    song_writer: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    genre: Option<String>,
    date_time: Option<String>,
    location: Option<String>,
    comment: Option<String>,
    user_specific: Option<String>,
}

// 音声パラメータ
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct AudioParams {
    sampling_frequency: Option<u32>,
    channel_count: Option<u8>,
    lfe_enabled: Option<bool>,
    
    channel_layout: Option<Vec<String>>,
    
    input_file_count: Option<usize>,
}

#[wasm_bindgen]
pub fn process_audio_wasm(
    file_data: &[u8],
    metadata_json: &str,
    params_json: &str
) -> Result<Vec<u8>, JsValue> {
    
    // 1. JSONパース
    let metadata: Metadata = serde_json::from_str(metadata_json).unwrap_or_default();
    
    // パース失敗時のエラーを詳細に出す
    let params: AudioParams = serde_json::from_str(params_json)
        .map_err(|e| JsValue::from_str(&format!("パラメータのJSONパースに失敗しました: {}", e)))?;

    let input_file_count = params.input_file_count.unwrap_or(1);
    
    // 2. メインロジック分岐
    let (mut interleaved_data, final_params) = if input_file_count > 1 {
        // Case A: 複数モノラルファイル
        handle_multi_mono_files(file_data, &params)?
    } else {
        // Case B: 単一ファイル
        handle_single_file(file_data, &params)?
    };

    // 3. サイクル総数を偶数にそろえる (仕様書 p.10)
    if let Some(ch) = final_params.channel_count {
        pad_to_even_cycles(&mut interleaved_data, ch as usize);
    }

    // 4. WSDファイル生成
    let wsd_data = create_wsd_file(&interleaved_data, &metadata, &final_params)
        .map_err(|e| JsValue::from_str(&e))?;

    Ok(wsd_data)
}

// 1bit の無音パターン (0 と 1 が均等)
const SILENCE_BYTE: u8 = 0x69;

// WSD の Stream Data 上のチャンネル順 (Ch_Asn の RBP48 MSB → RBP51 LSB)
const WSD_ORDER: [&str; 11] = [
    "lf", "lf-middle", "cf", "rf-middle", "rf", "lfe",
    "lr", "lr-middle", "cr", "rr-middle", "rr",
];

// サイクル(全チャンネル1バイトずつ)の総数が奇数なら、無音を1サイクル足す
fn pad_to_even_cycles(data: &mut Vec<u8>, channel_count: usize) {
    if channel_count == 0 || data.len() % channel_count != 0 {
        return;
    }
    if (data.len() / channel_count) % 2 == 1 {
        data.extend(std::iter::repeat(SILENCE_BYTE).take(channel_count));
    }
}

// Case A: マルチモノラルファイル処理
fn handle_multi_mono_files(
    file_data: &[u8],
    params: &AudioParams,
) -> Result<(Vec<u8>, AudioParams), JsValue> {
    
    let file_count = params.input_file_count.unwrap_or(1);
    
    // パラメータチェック
    let _ = params.sampling_frequency
        .ok_or_else(|| JsValue::from_str("マルチファイルモードではsamplingFrequencyが必須です"))?;
    let _ = params.channel_count
        .ok_or_else(|| JsValue::from_str("マルチファイルモードではchannelCountが必須です"))?;

    // データサイズチェック
    if file_data.len() % file_count != 0 {
        return Err(JsValue::from_str(&format!("データサイズ({})がファイル数({})で割り切れません", file_data.len(), file_count)));
    }

    let bytes_per_channel = file_data.len() / file_count;
    let mut channels = Vec::with_capacity(file_count);
    for i in 0..file_count {
        let start = i * bytes_per_channel;
        let end = start + bytes_per_channel;
        channels.push(&file_data[start..end]);
    }

    let interleaved = interleave_channels(&channels);
    Ok((interleaved, params.clone()))
}

fn interleave_channels(channels: &[&[u8]]) -> Vec<u8> {
    if channels.is_empty() { return Vec::new(); }
    let channel_count = channels.len();
    let bytes_per_channel = channels[0].len();
    let mut result = Vec::with_capacity(channel_count * bytes_per_channel);

    for byte_idx in 0..bytes_per_channel {
        for channel in channels {
            result.push(channel[byte_idx]);
        }
    }
    result
}

// Case B: 単一ファイル処理
fn handle_single_file(
    file_data: &[u8],
    params: &AudioParams,
) -> Result<(Vec<u8>, AudioParams), JsValue> {
    if file_data.len() < 4 { return Err(JsValue::from_str("ファイルが小さすぎます")); }
    
    match &file_data[0..4] {
        b"DSD " => convert_dsf(file_data).map_err(|e| JsValue::from_str(&e)),
        b"FRM8" => convert_dsdiff(file_data).map_err(|e| JsValue::from_str(&e)),
        b"RIFF" => Err(JsValue::from_str("WAVファイルは非対応です")),
        _ => {
            // Rawバイナリ
            if params.sampling_frequency.is_none() || params.channel_count.is_none() {
                return Err(JsValue::from_str("Rawバイナリの場合、samplingFrequencyとchannelCountの指定が必須です"));
            }
            Ok((file_data.to_vec(), params.clone()))
        }
    }
}

fn read_u16_be(data: &[u8], pos: usize) -> Option<u16> {
    data.get(pos..pos + 2).map(|b| u16::from_be_bytes(b.try_into().unwrap()))
}
fn read_u32_be(data: &[u8], pos: usize) -> Option<u32> {
    data.get(pos..pos + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()))
}
fn read_u64_be(data: &[u8], pos: usize) -> Option<u64> {
    data.get(pos..pos + 8).map(|b| u64::from_be_bytes(b.try_into().unwrap()))
}
fn read_u32_le(data: &[u8], pos: usize) -> Option<u32> {
    data.get(pos..pos + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}
fn read_u64_le(data: &[u8], pos: usize) -> Option<u64> {
    data.get(pos..pos + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

// 元データのチャンネル並び (WSD のチャンネル名) から、
// WSD_ORDER の順に並べたときの元インデックスの列を作る
fn wsd_order_indices(source_layout: &[String]) -> Result<Vec<usize>, String> {
    let order: Vec<usize> = WSD_ORDER.iter()
        .filter_map(|ch| source_layout.iter().position(|s| s == ch))
        .collect();
    if order.len() != source_layout.len() {
        return Err(format!("チャンネル配置が重複しているか不明です: {:?}", source_layout));
    }
    Ok(order)
}

// ---------------------------------------------------------------
// DSF (Sony)
//   全体がリトルエンディアン。各チャンクのサイズはヘッダー(12バイト)込み。
//   'DSD ' チャンク(28) → 'fmt ' チャンク(52) → 'data' チャンク → (メタデータ ID3)
//   音声は「チャンネルごとに Block size(通常4096)バイトのブロック」を交互に並べたもの。
//   Bits per sample = 1 のとき各バイトは LSB が先、8 のとき MSB が先。
// ---------------------------------------------------------------
fn convert_dsf(data: &[u8]) -> Result<(Vec<u8>, AudioParams), String> {
    let mut fmt: Option<&[u8]> = None;
    let mut audio: Option<&[u8]> = None;

    let mut pos = 0usize;
    while pos + 12 <= data.len() {
        let id = &data[pos..pos + 4];
        let size = read_u64_le(data, pos + 4).unwrap();
        if size < 12 { return Err("DSF: チャンクサイズが不正です".to_string()); }
        let end = pos.saturating_add(size as usize).min(data.len());
        match id {
            b"fmt " => fmt = Some(&data[pos..end]),
            b"data" => { audio = Some(&data[pos + 12..end]); break; }
            _ => {}
        }
        pos = end;
    }

    let fmt = fmt.ok_or("DSF: fmtチャンクが見つかりません")?;
    let audio = audio.ok_or("DSF: dataチャンクが見つかりません")?;
    if fmt.len() < 52 { return Err("DSF: fmtチャンクが短すぎます".to_string()); }

    let format_id = read_u32_le(fmt, 16).unwrap();
    let channel_type = read_u32_le(fmt, 20).unwrap();
    let channel_num = read_u32_le(fmt, 24).unwrap();
    let sampling_rate = read_u32_le(fmt, 28).unwrap();
    let bits_per_sample = read_u32_le(fmt, 32).unwrap();
    let sample_count = read_u64_le(fmt, 36).unwrap();
    let block_size = read_u32_le(fmt, 44).unwrap() as usize;

    if format_id != 0 {
        return Err(format!("DSF: 未対応のフォーマットです (Format ID: {})", format_id));
    }
    if bits_per_sample != 1 && bits_per_sample != 8 {
        return Err(format!("DSF: 未対応のビット順です (Bits per sample: {})", bits_per_sample));
    }
    if block_size == 0 { return Err("DSF: Block sizeが0です".to_string()); }
    if channel_num == 0 || channel_num > WSD_ORDER.len() as u32 {
        return Err(format!("DSF: 未対応のチャンネル数です ({}ch)", channel_num));
    }
    let channel_count = channel_num as usize;

    let source_layout = dsf_channel_layout(channel_type)
        .filter(|layout| layout.len() == channel_count)
        .or_else(|| create_default_channel_layout(channel_count as u8))
        .ok_or_else(|| format!(
            "DSF: 未対応のチャンネル構成です (Channel Type: {}, {}ch)", channel_type, channel_count
        ))?;
    let order = wsd_order_indices(&source_layout)?;

    // 1チャンネルあたりの有効バイト数。最後のブロックの詰め物は捨てる
    let blocks_per_channel = audio.len() / (block_size * channel_count);
    let available = blocks_per_channel * block_size;
    let bytes_per_channel = (sample_count.div_ceil(8) as usize).min(available);

    let lsb_first = bits_per_sample == 1;
    let mut result = Vec::with_capacity(bytes_per_channel * channel_count);
    for i in 0..bytes_per_channel {
        let block = i / block_size;
        let offset = i % block_size;
        for &src_ch in &order {
            let byte = audio[(block * channel_count + src_ch) * block_size + offset];
            result.push(if lsb_first { byte.reverse_bits() } else { byte });
        }
    }

    log(&format!("DSF解析成功: {}Hz, {}ch, {}サンプル", sampling_rate, channel_count, sample_count));

    let mut params = AudioParams::default();
    params.sampling_frequency = Some(sampling_rate);
    params.channel_count = Some(channel_count as u8);
    params.channel_layout = Some(source_layout);
    Ok((result, params))
}

// DSF の Channel Type → WSD のチャンネル名 (DSF 上の並び順)
fn dsf_channel_layout(channel_type: u32) -> Option<Vec<String>> {
    let layout: &[&str] = match channel_type {
        1 => &["cf"],
        2 => &["lf", "rf"],
        3 => &["lf", "rf", "cf"],
        4 => &["lf", "rf", "lr", "rr"],
        5 => &["lf", "rf", "cf", "lfe"],
        6 => &["lf", "rf", "cf", "lr", "rr"],
        7 => &["lf", "rf", "cf", "lfe", "lr", "rr"],
        _ => return None,
    };
    Some(layout.iter().map(|s| s.to_string()).collect())
}

// ---------------------------------------------------------------
// DSDIFF (Philips)
//   全体がビッグエンディアン。'FRM8' + Size(8) + 'DSD ' + チャンク列。
//   各チャンクは ID(4) + Size(8, ヘッダーを含まない) + Data で、
//   Size が奇数なら後ろに詰め物が1バイト入る。
//   'PROP' チャンク('SND ' + FS, CHNL, CMPR, ...)の後に 'DSD '(無圧縮)
//   または 'DST '(圧縮)の音声チャンクが来る。
//   音声は1バイトごとにチャンネルを交互に並べたもので、MSB が先 (WSD と同じ)。
// ---------------------------------------------------------------

// data[start..end] のチャンク列を (ID, 中身) で返す
fn iff_chunks(data: &[u8], start: usize, end: usize) -> Result<Vec<(&[u8], &[u8])>, String> {
    let mut chunks = Vec::new();
    let mut pos = start;
    while pos + 12 <= end {
        let id = &data[pos..pos + 4];
        let size = read_u64_be(data, pos + 4).unwrap();
        let body_start = pos + 12;
        let body_end = body_start.saturating_add(size as usize).min(end);
        chunks.push((id, &data[body_start..body_end]));
        // 奇数サイズの詰め物を飛ばす
        pos = body_end.saturating_add((size % 2) as usize);
    }
    Ok(chunks)
}

fn convert_dsdiff(data: &[u8]) -> Result<(Vec<u8>, AudioParams), String> {
    if data.len() < 16 || &data[12..16] != b"DSD " {
        return Err("DSDIFF: FRM8のフォームタイプが'DSD 'ではありません".to_string());
    }
    let form_size = read_u64_be(data, 4).unwrap();
    let form_end = 12usize.saturating_add(form_size as usize).min(data.len());

    let mut sampling_rate: Option<u32> = None;
    let mut source_layout: Option<Vec<String>> = None;
    let mut channel_count = 0usize;
    let mut audio: Option<&[u8]> = None;

    for (id, body) in iff_chunks(data, 16, form_end)? {
        match id {
            b"PROP" => {
                if body.len() < 4 || &body[0..4] != b"SND " { continue; }
                for (sub_id, sub) in iff_chunks(body, 4, body.len())? {
                    match sub_id {
                        b"FS  " => sampling_rate = read_u32_be(sub, 0),
                        b"CHNL" => {
                            let n = read_u16_be(sub, 0).ok_or("DSDIFF: CHNLチャンク破損")? as usize;
                            let ids = sub.get(2..2 + 4 * n).ok_or("DSDIFF: CHNLチャンク破損")?;
                            channel_count = n;
                            source_layout = ids.chunks_exact(4)
                                .map(|id| dsdiff_channel_name(id).map(|s| s.to_string()))
                                .collect();
                        }
                        b"CMPR" => {
                            let kind = sub.get(0..4).ok_or("DSDIFF: CMPRチャンク破損")?;
                            if kind == b"DST " {
                                return Err("DSDIFF: DST圧縮のファイルは未対応です".to_string());
                            }
                            if kind != b"DSD " {
                                return Err(format!(
                                    "DSDIFF: 未対応の圧縮形式です ({})", String::from_utf8_lossy(kind)
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
            b"DSD " => audio = Some(body),
            b"DST " => return Err("DSDIFF: DST圧縮のファイルは未対応です".to_string()),
            _ => {}
        }
    }

    let sampling_rate = sampling_rate.ok_or("DSDIFF: FSチャンクが見つかりません")?;
    if channel_count == 0 { return Err("DSDIFF: CHNLチャンクが見つかりません".to_string()); }
    if channel_count > WSD_ORDER.len() {
        return Err(format!("DSDIFF: 未対応のチャンネル数です ({}ch)", channel_count));
    }
    let audio = audio.ok_or("DSDIFF: 音声データ(DSD)チャンクが見つかりません")?;

    // チャンネルIDが不明・重複なら、1ch/2ch に限り既定の配置を使う
    let source_layout = source_layout
        .filter(|layout| wsd_order_indices(layout).is_ok())
        .or_else(|| create_default_channel_layout(channel_count as u8))
        .ok_or_else(|| format!("DSDIFF: 未対応のチャンネル構成です ({}ch)", channel_count))?;
    let order = wsd_order_indices(&source_layout)?;

    // WSD の順に並べ替える (半端な最後のサイクルは捨てる)
    let mut result = Vec::with_capacity(audio.len());
    for cycle in audio.chunks_exact(channel_count) {
        result.extend(order.iter().map(|&src| cycle[src]));
    }

    log(&format!("DSDIFF解析成功: {}Hz, {}ch", sampling_rate, channel_count));

    let mut params = AudioParams::default();
    params.sampling_frequency = Some(sampling_rate);
    params.channel_count = Some(channel_count as u8);
    params.channel_layout = Some(source_layout);
    Ok((result, params))
}

// DSDIFF の CHNL チャンネルID → WSD のチャンネル名
fn dsdiff_channel_name(id: &[u8]) -> Option<&'static str> {
    match id {
        b"SLFT" | b"MLFT" => Some("lf"),
        b"SRGT" | b"MRGT" => Some("rf"),
        b"C   " => Some("cf"),
        b"LFE " => Some("lfe"),
        b"LS  " => Some("lr"),
        b"RS  " => Some("rr"),
        _ => None,
    }
}

// デフォルト配置 (配列版)
fn create_default_channel_layout(channel_count: u8) -> Option<Vec<String>> {
    match channel_count {
        1 => Some(vec!["cf".to_string()]),
        2 => Some(vec!["lf".to_string(), "rf".to_string()]),
        _ => None
    }
}

// WSD生成
fn create_wsd_file(
    stream_data: &[u8],
    metadata: &Metadata,
    params: &AudioParams,
) -> Result<Vec<u8>, String> {
    
    let fs = params.sampling_frequency.ok_or("サンプリングレート設定なし")?;
    let ch = params.channel_count.ok_or("チャンネル数設定なし")?;

    let total_size = 2048 + stream_data.len();
    let mut output = Vec::with_capacity(total_size);

    // General Info
    output.extend_from_slice(b"1bit");
    output.extend_from_slice(&[0; 4]);
    output.push(0x11); output.push(0); output.extend_from_slice(&[0; 2]);
    let file_sz = 2048u64 + stream_data.len() as u64;
    output.extend_from_slice(&file_sz.to_be_bytes()[4..8]); 
    output.extend_from_slice(&file_sz.to_be_bytes()[0..4]); 
    output.extend_from_slice(&[0x00, 0x00, 0x00, 0x80]);
    output.extend_from_slice(&[0x00, 0x00, 0x08, 0x00]);
    output.extend_from_slice(&[0; 4]);

    // Data Spec Info
    let total_bits = (stream_data.len() as u64) * 8;
    let bits_per_sec = (fs as u64) * (ch as u64);
    let total_seconds = total_bits as f64 / bits_per_sec as f64;
    output.push(0);
    output.push(to_bcd((total_seconds % 60.0) as u8));
    output.push(to_bcd(((total_seconds % 3600.0) / 60.0) as u8));
    output.push(to_bcd((total_seconds / 3600.0) as u8));
    
    output.extend_from_slice(&fs.to_be_bytes());
    output.extend_from_slice(&[0; 4]);
    output.push(ch);
    output.extend_from_slice(&[0; 3]);

    // Ch_Asn (文字列配列からビットフラグへ)
    let mut ch_asn_front = 0x00;
    if let Some(layout) = &params.channel_layout {
        if layout.iter().any(|s| s == "lf") { ch_asn_front |= 1 << 6; }
        if layout.iter().any(|s| s == "lf-middle") { ch_asn_front |= 1 << 5; }
        if layout.iter().any(|s| s == "cf") { ch_asn_front |= 1 << 4; }
        if layout.iter().any(|s| s == "rf-middle") { ch_asn_front |= 1 << 3; }
        if layout.iter().any(|s| s == "rf") { ch_asn_front |= 1 << 2; }
        if layout.iter().any(|s| s == "lfe") { ch_asn_front |= 1 << 0; }
    }
    output.push(ch_asn_front);
    output.extend_from_slice(&[0; 2]);

    let mut ch_asn_rear = 0x00;
    if let Some(layout) = &params.channel_layout {
        if layout.iter().any(|s| s == "lr") { ch_asn_rear |= 1 << 6; }
        if layout.iter().any(|s| s == "lr-middle") { ch_asn_rear |= 1 << 5; }
        if layout.iter().any(|s| s == "cr") { ch_asn_rear |= 1 << 4; }
        if layout.iter().any(|s| s == "rr-middle") { ch_asn_rear |= 1 << 3; }
        if layout.iter().any(|s| s == "rr") { ch_asn_rear |= 1 << 2; }
    }
    output.push(ch_asn_rear);
    
    output.extend_from_slice(&[0; 12]);
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(&[0; 16]);
    output.extend_from_slice(&[0; 40]);

    create_text_data(&mut output, metadata)?;
    output.extend_from_slice(stream_data);
    Ok(output)
}

fn to_bcd(val: u8) -> u8 { ((val / 10) << 4) | (val % 10) }

fn create_text_data(output: &mut Vec<u8>, metadata: &Metadata) -> Result<(), String> {
    let fields = [
        (&metadata.title, 128), (&metadata.composer, 128), (&metadata.song_writer, 128),
        (&metadata.artist, 128), (&metadata.album, 128), (&metadata.genre, 32),
        (&metadata.date_time, 32), (&metadata.location, 32),
        (&metadata.comment, 512), (&metadata.user_specific, 512),
    ];
    for (text_opt, max_length) in fields.iter() {
        let text = text_opt.as_deref().unwrap_or("");
        let mut field_bytes = text.as_bytes().to_vec();
        field_bytes.truncate(*max_length);
        field_bytes.resize(*max_length, 0x20);
        output.extend_from_slice(&field_bytes);
    }
    let remaining = 1920 - 1760;
    output.extend_from_slice(&vec![0x20; remaining]);
    Ok(())
}

// JSへ返すためのメタデータ構造体（読み取り専用）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub composer: String,
    pub genre: String,
    pub date_time: String, 
    pub has_data: bool,    
}

impl Default for ExtractedMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            composer: String::new(),
            genre: String::new(),
            date_time: String::new(),
            has_data: false,
        }
    }
}

/// ファイルからメタデータを抽出する関数
#[wasm_bindgen]
pub fn get_file_metadata_wasm(file_data: &[u8]) -> Result<JsValue, JsValue> {
    let mut meta = ExtractedMetadata::default();

    if file_data.len() > 4 && &file_data[0..4] == b"DSD " {
        // DSFファイルの場合、ID3タグを探す
        // DSD Chunk (28bytes) の offset 20 にメタデータへのポインタがある
        if file_data.len() >= 28 {
            let meta_ptr_bytes: [u8; 8] = file_data[20..28].try_into().unwrap();
            let meta_offset = u64::from_le_bytes(meta_ptr_bytes) as usize;

            if meta_offset > 0 && meta_offset < file_data.len() {
                // ID3解析を試みる
                parse_id3_simple(&file_data[meta_offset..], &mut meta);
            }
        }
    }
    // (ここで拡張可能)

    Ok(serde_wasm_bindgen::to_value(&meta).unwrap())
}

// 簡易的なID3v2パーサー 
fn parse_id3_simple(data: &[u8], meta: &mut ExtractedMetadata) {
    if data.len() < 10 || &data[0..3] != b"ID3" {
        return;
    }

    // ID3 Header (10 bytes)
    // Size is synchsafe integer (bit 7 is ignored)
    let size_bytes = &data[6..10];
    let tag_size = ((size_bytes[0] as usize) << 21)
                 | ((size_bytes[1] as usize) << 14)
                 | ((size_bytes[2] as usize) << 7)
                 | (size_bytes[3] as usize);
    
    let mut pos = 10;
    let limit = std::cmp::min(10 + tag_size, data.len());

    while pos < limit {
        // Frame Header (10 bytes): ID(4) + Size(4) + Flags(2)
        if pos + 10 > limit { break; }
        
        let frame_id = &data[pos..pos+4];
        if frame_id[0] == 0 { break; } // Padding reached

        // Frame Size (Synchsafe in v2.4, regular integer in v2.3 usually)
        let fsize = u32::from_be_bytes(data[pos+4..pos+8].try_into().unwrap()) as usize;
        
        if pos + 10 + fsize > limit { break; }

        let frame_content = &data[pos+10..pos+10+fsize];
        
        // テキストフレームの読み取り (Encoding byte + Text)
        if fsize > 1 {
            let encoding = frame_content[0]; // 0=ISO-8859-1, 1=UTF-16BOM, 3=UTF-8
            let text_bytes = &frame_content[1..];
            
            let text = decode_text(encoding, text_bytes);

            match frame_id {
                b"TIT2" => meta.title = text,    // Title
                b"TPE1" => meta.artist = text,   // Artist
                b"TALB" => meta.album = text,    // Album
                b"TCOM" => meta.composer = text, // Composer
                b"TCON" => meta.genre = text,    // Genre
                b"TYER" | b"TDRC" => meta.date_time = text, // Year
                _ => {}
            }
        }

        pos += 10 + fsize;
    }
    
    if !meta.title.is_empty() || !meta.artist.is_empty() {
        meta.has_data = true;
    }
}

// テキストデコードヘルパー
fn decode_text(encoding: u8, data: &[u8]) -> String {
    // 終端のNULLバイトをトリム
    let clean_data = data.split(|&b| b == 0).next().unwrap_or(data);
    
    match encoding {
        0 => String::from_utf8_lossy(clean_data).to_string(), // ISO-8859-1 (Latin1) -> 近似的にUTF8扱い
        1 => {
            // UTF-16 with BOM
            if clean_data.len() >= 2 {
                let u16_vec: Vec<u16> = clean_data
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]])) 
                    .collect();
                String::from_utf16_lossy(&u16_vec)
            } else { String::new() }
        },
        3 => String::from_utf8_lossy(clean_data).to_string(), // UTF-8
        _ => String::from_utf8_lossy(clean_data).to_string(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn ch_asn(layout: &[&str]) -> (u8, u8) {
        let params = AudioParams {
            sampling_frequency: Some(2_822_400),
            channel_count: Some(layout.len() as u8),
            channel_layout: Some(layout.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        };
        let out = create_wsd_file(&[0; 8], &Metadata::default(), &params).unwrap();
        (out[48], out[51])
    }

    // 仕様書 p.7: 2ch は RBP48=44h, RBP51=00h
    #[test]
    fn ch_asn_stereo() {
        assert_eq!(ch_asn(&["lf", "rf"]), (0x44, 0x00));
    }

    // 仕様書 p.7: 4ch は RBP48=44h, RBP51=44h
    #[test]
    fn ch_asn_quad() {
        assert_eq!(ch_asn(&["lf", "rf", "lr", "rr"]), (0x44, 0x44));
    }

    // 仕様書 p.7: 5.1ch は RBP48=55h, RBP51=44h
    #[test]
    fn ch_asn_surround_5_1() {
        assert_eq!(ch_asn(&["lf", "cf", "rf", "lfe", "lr", "rr"]), (0x55, 0x44));
    }

    // 仕様書 p.7: Multiple Mono の Cf は RBP48=10h
    #[test]
    fn ch_asn_mono_cf() {
        assert_eq!(ch_asn(&["cf"]), (0x10, 0x00));
    }

    // 仕様書 p.10: サイクル総数は偶数
    #[test]
    fn pad_odd_cycles_with_silence() {
        let mut data = vec![1, 2, 3, 4, 5, 6]; // 2ch × 3サイクル
        pad_to_even_cycles(&mut data, 2);
        assert_eq!(data, vec![1, 2, 3, 4, 5, 6, 0x69, 0x69]);
    }

    #[test]
    fn keep_even_cycles() {
        let mut data = vec![1, 2, 3, 4]; // 2ch × 2サイクル
        pad_to_even_cycles(&mut data, 2);
        assert_eq!(data, vec![1, 2, 3, 4]);
    }

    fn wsd_from(file: &[u8]) -> Vec<u8> {
        let (mut data, params) = handle_single_file(file, &AudioParams::default()).unwrap();
        pad_to_even_cycles(&mut data, params.channel_count.unwrap() as usize);
        create_wsd_file(&data, &Metadata::default(), &params).unwrap()
    }

    // DSF を組み立てる。channels はチャンネルごとのバイト列 (ファイルに格納される形のまま)
    fn dsf_file(channel_type: u32, bits_per_sample: u32, block_size: usize,
                sample_count: u64, channels: &[Vec<u8>]) -> Vec<u8> {
        let ch = channels.len();
        let blocks = channels.iter().map(|c| c.len().div_ceil(block_size)).max().unwrap();
        let mut audio = Vec::new();
        for b in 0..blocks {
            for c in channels {
                let mut block: Vec<u8> = c.iter().skip(b * block_size).take(block_size).copied().collect();
                block.resize(block_size, 0x00); // 最後のブロックの詰め物
                audio.extend(block);
            }
        }

        let mut f = Vec::new();
        f.extend_from_slice(b"DSD ");
        f.extend_from_slice(&28u64.to_le_bytes());
        f.extend_from_slice(&0u64.to_le_bytes()); // total file size (未使用)
        f.extend_from_slice(&0u64.to_le_bytes()); // metadata pointer
        f.extend_from_slice(b"fmt ");
        f.extend_from_slice(&52u64.to_le_bytes());
        f.extend_from_slice(&1u32.to_le_bytes()); // format version
        f.extend_from_slice(&0u32.to_le_bytes()); // format id (DSD raw)
        f.extend_from_slice(&channel_type.to_le_bytes());
        f.extend_from_slice(&(ch as u32).to_le_bytes());
        f.extend_from_slice(&2_822_400u32.to_le_bytes());
        f.extend_from_slice(&bits_per_sample.to_le_bytes());
        f.extend_from_slice(&sample_count.to_le_bytes());
        f.extend_from_slice(&(block_size as u32).to_le_bytes());
        f.extend_from_slice(&0u32.to_le_bytes()); // reserved
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(12 + audio.len() as u64).to_le_bytes());
        f.extend(audio);
        f.extend_from_slice(b"ID3\x03\x00\x00\x00\x00\x00\x00"); // 後ろのメタデータ (音声に含めない)
        f
    }

    // DSF ステレオ: ブロックを解いて1バイト交互に、LSB先→MSB先に反転、詰め物とID3は捨てる
    #[test]
    fn dsf_stereo_deinterleaves_and_reverses_bits() {
        let l = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let r = vec![0x10, 0x20, 0x30, 0x40, 0x50, 0x60];
        // block_size 4 → L[0..4] R[0..4] L[4..6]+詰め物 R[4..6]+詰め物
        let (data, params) = convert_dsf(&dsf_file(2, 1, 4, 48, &[l.clone(), r.clone()])).unwrap();

        let expected: Vec<u8> = l.iter().zip(&r)
            .flat_map(|(a, b)| [a.reverse_bits(), b.reverse_bits()])
            .collect();
        assert_eq!(data, expected);
        assert_eq!(params.channel_count, Some(2));
        assert_eq!(params.sampling_frequency, Some(2_822_400));
    }

    // Sample count で切り詰め、奇数サイクルは無音で埋める
    #[test]
    fn dsf_trims_by_sample_count() {
        let l = vec![0x80; 8];
        let r = vec![0x80; 8];
        let wsd = wsd_from(&dsf_file(2, 8, 4096, 40, &[l, r])); // 40サンプル = 5バイト
        assert_eq!((wsd[44], wsd[48], wsd[51]), (2, 0x44, 0x00));
        assert_eq!(&wsd[2048..], &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x69, 0x69]);
    }

    // DSF 5.1ch (Channel Type 7: FL FR C LFE BL BR) → WSD 順 Lf Cf Rf LFE Lr Rr
    #[test]
    fn dsf_5_1_reorders_to_wsd() {
        let channels: Vec<Vec<u8>> = (1..=6).map(|c| vec![c as u8, 0x10 + c as u8]).collect();
        let file = dsf_file(7, 8, 4096, 16, &channels);
        let (data, _) = convert_dsf(&file).unwrap();
        assert_eq!(data, vec![1, 3, 2, 4, 5, 6, 0x11, 0x13, 0x12, 0x14, 0x15, 0x16]);

        let wsd = wsd_from(&file);
        assert_eq!((wsd[44], wsd[48], wsd[51]), (6, 0x55, 0x44));
    }

    // DSF 3ch (Channel Type 3: L, R, C)
    #[test]
    fn dsf_3ch_sets_ch_asn() {
        let channels = vec![vec![0u8; 2]; 3];
        let wsd = wsd_from(&dsf_file(3, 1, 4096, 16, &channels));
        assert_eq!((wsd[44], wsd[48], wsd[51]), (3, 0x54, 0x00));
    }

    fn iff_chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut c = id.to_vec();
        c.extend_from_slice(&(body.len() as u64).to_be_bytes());
        c.extend_from_slice(body);
        if body.len() % 2 == 1 { c.push(0); } // 奇数サイズの詰め物
        c
    }

    // 仕様どおりの構成: FVER, PROP(SND : FS, CHNL, CMPR), COMT, DSD, DIIN
    fn dsdiff_file(ids: &[&[u8; 4]], compression: &[u8; 4], audio: &[u8]) -> Vec<u8> {
        let mut chnl = (ids.len() as u16).to_be_bytes().to_vec();
        for id in ids { chnl.extend_from_slice(*id); }
        let mut cmpr = compression.to_vec();
        cmpr.push(14);
        cmpr.extend_from_slice(b"not compressed"); // 4+1+14 = 19 (奇数)

        let mut prop = b"SND ".to_vec();
        prop.extend(iff_chunk(b"FS  ", &2_822_400u32.to_be_bytes()));
        prop.extend(iff_chunk(b"CHNL", &chnl));
        prop.extend(iff_chunk(b"CMPR", &cmpr));

        let mut body = b"DSD ".to_vec();
        body.extend(iff_chunk(b"FVER", &[1, 5, 0, 0]));
        body.extend(iff_chunk(b"PROP", &prop));
        body.extend(iff_chunk(b"COMT", b"odd")); // 奇数サイズ
        body.extend(iff_chunk(compression, audio));
        body.extend(iff_chunk(b"DIIN", &iff_chunk(b"DITI", b"\x00\x00\x00\x04abcd")));

        let mut f = b"FRM8".to_vec();
        f.extend_from_slice(&(body.len() as u64).to_be_bytes());
        f.extend(body);
        f
    }

    // DSDIFF ステレオ: CMPR の 'DSD ' に惑わされず、音声チャンクだけを取り出す
    #[test]
    fn dsdiff_stereo_reads_only_audio_chunk() {
        let ids: [&[u8; 4]; 2] = [b"SLFT", b"SRGT"];
        let wsd = wsd_from(&dsdiff_file(&ids, b"DSD ", &[0xA1, 0xB1, 0xA2, 0xB2, 0xA3, 0xB3]));
        assert_eq!((wsd[44], wsd[48], wsd[51]), (2, 0x44, 0x00));
        assert_eq!(&wsd[2048..], &[0xA1, 0xB1, 0xA2, 0xB2, 0xA3, 0xB3, 0x69, 0x69]);
    }

    // DSDIFF 5.1ch: MLFT MRGT C LFE LS RS → WSD 順 Lf Cf Rf LFE Lr Rr に並べ替える
    #[test]
    fn dsdiff_5_1_reorders_to_wsd() {
        let ids: [&[u8; 4]; 6] = [b"MLFT", b"MRGT", b"C   ", b"LFE ", b"LS  ", b"RS  "];
        let audio = [1, 2, 3, 4, 5, 6, 11, 12, 13, 14, 15, 16];
        let file = dsdiff_file(&ids, b"DSD ", &audio);
        let (data, _) = convert_dsdiff(&file).unwrap();
        assert_eq!(data, vec![1, 3, 2, 4, 5, 6, 11, 13, 12, 14, 15, 16]);

        let wsd = wsd_from(&file);
        assert_eq!((wsd[44], wsd[48], wsd[51]), (6, 0x55, 0x44));
    }

    // DST 圧縮はエラーにする
    #[test]
    fn dsdiff_dst_is_rejected() {
        let ids: [&[u8; 4]; 2] = [b"SLFT", b"SRGT"];
        let err = convert_dsdiff(&dsdiff_file(&ids, b"DST ", &[0; 4])).unwrap_err();
        assert!(err.contains("DST"), "{}", err);
    }
}
