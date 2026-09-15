use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::str;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

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
    let (interleaved_data, final_params) = if input_file_count > 1 {
        // Case A: 複数モノラルファイル
        handle_multi_mono_files(file_data, &params)?
    } else {
        // Case B: 単一ファイル
        handle_single_file(file_data, &params)?
    };

    // 3. WSDファイル生成
    let wsd_data = create_wsd_file(&interleaved_data, &metadata, &final_params)
        .map_err(|e| JsValue::from_str(&e))?;

    Ok(wsd_data)
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
        b"DSD " => {
            let (dsd_data, detected_params) = parse_dsf_header(file_data)?;
            Ok((dsd_data.to_vec(), detected_params))
        },
        b"FRM8" => {
            let (dsd_data, detected_params) = parse_dsdiff_header(file_data)?;
            Ok((dsd_data.to_vec(), detected_params))
        },
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

// ヘッダー解析 (DSF)
fn parse_dsf_header(data: &[u8]) -> Result<(&[u8], AudioParams), JsValue> {
    if data.len() < 92 { return Err(JsValue::from_str("DSFファイルエラー")); }
    
    let ch_num_bytes: [u8; 4] = data[52..56].try_into().unwrap();
    let channel_count = u32::from_le_bytes(ch_num_bytes) as u8;
    
    let fs_bytes: [u8; 4] = data[56..60].try_into().unwrap();
    let sampling_rate = u32::from_le_bytes(fs_bytes);
    
    let data_pos = data.windows(4).position(|w| w == b"data")
        .map(|p| p + 12).ok_or_else(|| JsValue::from_str("dataチャンクなし"))?;
        
    let mut params = AudioParams::default();
    params.sampling_frequency = Some(sampling_rate);
    params.channel_count = Some(channel_count);
    params.channel_layout = create_default_channel_layout(channel_count);
    
    if data_pos >= data.len() { return Err(JsValue::from_str("データ位置異常")); }
    Ok((&data[data_pos..], params))
}

/// DSDIFFヘッダー解析（完全版）
fn parse_dsdiff_header(data: &[u8]) -> Result<(&[u8], AudioParams), JsValue> {
    if data.len() < 28 {
        return Err(JsValue::from_str("DSDIFFファイルが短すぎます"));
    }

    // DSDIFF構造: 'FRM8' + size(8) + 'DSD ' + chunks...
    // チャンク構造: ID(4) + Size(8) + Data...
    
    // 1. FS (sampling rate) を探す ('FS  ' chunk)
    let fs_pos = data.windows(4)
        .position(|w| w == b"FS  ")
        .ok_or_else(|| JsValue::from_str("DSDIFF: FSチャンクが見つかりません"))?;
    
    let fs_val_pos = fs_pos + 12; // ID(4)+Size(8)=12
    if fs_val_pos + 4 > data.len() { return Err(JsValue::from_str("DSDIFF: FSチャンク破損")); }
    let sampling_rate = u32::from_be_bytes(data[fs_val_pos..fs_val_pos+4].try_into().unwrap());

    // 2. CHNL (channel count) を探す ('CHNL' chunk)
    let chnl_pos = data.windows(4)
        .position(|w| w == b"CHNL")
        .ok_or_else(|| JsValue::from_str("DSDIFF: CHNLチャンクが見つかりません"))?;
    
    let num_ch_pos = chnl_pos + 12;
    if num_ch_pos + 2 > data.len() { return Err(JsValue::from_str("DSDIFF: CHNLチャンク破損")); }
    let channel_count = u16::from_be_bytes(data[num_ch_pos..num_ch_pos+2].try_into().unwrap()) as u8;

    // 3. DSD (audio data) を探す ('DSD ' chunk)
    let dsd_data_start = data.windows(4).enumerate()
        .filter(|(_, w)| *w == b"DSD ")
        .map(|(i, _)| i)
        .find(|&i| i > 12) // 先頭のID以外を探す
        .map(|i| i + 12); // ID(4)+Size(8)=12

    let data_start = dsd_data_start.ok_or_else(|| JsValue::from_str("DSDIFF: 音声データ(DSD)チャンクが見つかりません"))?;

    if data_start >= data.len() {
        return Err(JsValue::from_str("DSDIFF: データ開始位置異常"));
    }

    log(&format!("DSDIFF解析成功: {}Hz, {}ch", sampling_rate, channel_count));

    let mut params = AudioParams::default();
    params.sampling_frequency = Some(sampling_rate);
    params.channel_count = Some(channel_count);
    params.channel_layout = create_default_channel_layout(channel_count);

    Ok((&data[data_start..], params))
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
    let mut ch_asn_front = 0x01;
    if let Some(layout) = &params.channel_layout {
        if layout.iter().any(|s| s == "lf") { ch_asn_front |= 1 << 6; }
        if layout.iter().any(|s| s == "lf-middle") { ch_asn_front |= 1 << 5; }
        if layout.iter().any(|s| s == "cf") { ch_asn_front |= 1 << 4; }
        if layout.iter().any(|s| s == "rf-middle") { ch_asn_front |= 1 << 3; }
        if layout.iter().any(|s| s == "rf") { ch_asn_front |= 1 << 2; }
        if layout.iter().any(|s| s == "lfe") { ch_asn_front |= 1 << 1; }
    }
    output.push(ch_asn_front);
    output.extend_from_slice(&[0; 2]);

    let mut ch_asn_rear = 0x01;
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