import tkinter as tk
from tkinter import ttk
from tkinter import filedialog

class WSDConverter:
  def __init__(self, root):
    self.root = root
    self.root.title("Creating wsd file")
    self.file_references = []
    self.options = ["Lf", "Lf-middle", "Cf", "Rf-middle", "Rf", "Lr", "Lr-middle", "Cr", "Rr-middle", "Rr"]
    self.fields = [
      ("Title：", 128),
      ("Composer：", 128),
      ("Song Writer：", 128),
      ("Artist：", 128),
      ("Album：", 128),
      ("Genre：", 32),
      ("Data&Time：", 32),
      ("Location", 32),
      ("Comment：", 512),
      ("User Specific：", 512)
    ]
    self.checkbox = []
    self.DataFilepaths = []
    self.Ch_N = 0
    self.binaries = []
    self.Stream_Data = b""
    self.Text_binary = b""
    self.Text = []
    self.create_widgets()

  def create_widgets(self): #make window
    self.frame = ttk.Frame(self.root)
    self.frame.grid(row=1, column=0, sticky=tk.W)
    self.create_chN_frame()
    self.create_lfe_frame()
    self.create_chAsn_frame()
    self.create_fs_frame()
    self.create_entry_frame()
    self.create_button_frame()

  def create_chN_frame(self): #input chN
    frame_num = ttk.Frame(self.frame)
    label_a = tk.Label(frame_num, text="The number of channel：")
    label_a.pack(side="left", anchor="w")
    self.Ch_N = ttk.Combobox(frame_num, values=[1, 2, 3, 4, 5], width=1, state="readonly")
    self.Ch_N.pack(side="left", anchor="w")
    self.Ch_N.current(1)
    frame_num.pack(side="top", anchor="w", padx=10, pady=10)

  def create_lfe_frame(self):#LFE on/off
    frame_LFE = ttk.Frame(self.frame)
    tk.Label(frame_LFE, text="LFE：").pack(side="left", anchor="w")
    self.channel_var = tk.StringVar()
    channels = ['Yes', 'No']
    for channel in channels:
      radio_button = ttk.Radiobutton(frame_LFE, text=channel, variable=self.channel_var, value=channel, command=self.update_file_references)
      radio_button.pack(side="left", anchor="w",padx=(0,10))
    self.channel_var.set('No')
    frame_LFE.pack(side="top", anchor="w", padx=10, pady=10)

  def create_chAsn_frame(self):#チャンネル配置フレーム
    frame_channel = ttk.Frame(self.frame)
    ttk.Label(frame_channel, text="Channel Assignment：").pack(side="top", anchor="w")
    row1_frame = ttk.Frame(frame_channel)
    row1_frame.pack(side="top", anchor="w")
    row2_frame = ttk.Frame(frame_channel)
    row2_frame.pack(side="top", anchor="w")
    for option in self.options:
      num = tk.IntVar()
      DataFilepath = tk.StringVar()
      self.checkbox.append(num)
      self.DataFilepaths.append(DataFilepath)
    self.DataFilepaths.append(tk.StringVar())
    for index, (option, num) in enumerate(zip(self.options, self.checkbox)):
      checkbox = tk.Checkbutton(row1_frame if index < 5 else row2_frame, text=option, variable=num, command=self.update_file_references)
      checkbox.pack(side="left", anchor="w")
    frame_channel.pack(side="top", anchor="w", padx=10, pady=10)
    self.framemain = ttk.Frame(self.root)
    self.framemain.grid(row=1, column=2, sticky=tk.W)

  def create_fs_frame(self):#choose fs
    frame_fs = ttk.Frame(self.frame)
    tk.Label(frame_fs, text="Sampling rate：").pack(side="left", anchor="w")
    self.Fs = ttk.Combobox(frame_fs, values=[1411200, 2822400, 5644800, 11289600, "Other"], width=8, state="readonly")
    self.Fs.bind("<<ComboboxSelected>>", self.on_combobox_select)
    self.Fs.pack(side="left", anchor="w")
    tk.Label(frame_fs, text="Hz").pack(side="left", anchor="w")
    self.Fs.current(2)
    frame_fs.pack(side="top", anchor="w", padx=10, pady=10)

  def create_entry_frame(self):#choose filepath
    self.text_entries = []
    frame_entry = ttk.Frame(self.root)
    frame_entry.grid(row=1, column=3, sticky=tk.W)
    
    row_index = 1
    for label_text, width in self.fields:
      label = tk.Label(frame_entry, text=label_text)
      label.grid(row=row_index, column=0, sticky="w", padx=10, pady=(10, 0))
      entry = self.create_text_with_dynamic_height(frame_entry, 32, width)
      entry.grid(row=row_index + 1, column=0, sticky="w", padx=10)
      self.text_entries.append(entry) # Save the text widget reference
      row_index += 2


  def create_button_frame(self):#run button
    framebutton = ttk.Frame(self.root)
    framebutton.grid(row=10, column=2, sticky=tk.W)
    button1 = ttk.Button(framebutton, text="Run", command=self.conduct_main)
    button1.pack(fill="x", pady=10, side=tk.BOTTOM)

  def filedialog_clicked(self, index):#ファイル選択
    filename = filedialog.askopenfilename()
    if filename:
      self.DataFilepaths[index].set(filename)

  def update_file_references(self): # チャンネル配置を選択したときにファイルパスを選択できるようにするフレーム
    for label, entry, button in self.file_references:
      label.destroy()
      entry.destroy()
      button.destroy()
    self.file_references = []
    for i, num in enumerate(self.checkbox):
      if num.get() == 1:
        label_text = f"Audio data of {self.options[i]}： "
        label = ttk.Label(self.framemain, text=label_text)
        label.pack(anchor=tk.W, pady=(10, 0))
        entry = ttk.Entry(self.framemain, textvariable=self.DataFilepaths[i], width=30)
        entry.pack(anchor=tk.W)
        button = ttk.Button(self.framemain, text="Browse", command=lambda i=i: self.filedialog_clicked(i))
        button.pack(anchor=tk.W)
        self.file_references.append((label, entry, button))
      else:
        self.DataFilepaths[i].set("")

    if self.channel_var.get() == 'Yes':
      label_text = f"Audio data of LFE： "
      label = ttk.Label(self.framemain, text=label_text)
      label.pack(side="top", anchor="w")
      entry = ttk.Entry(self.framemain, textvariable=self.DataFilepaths[10], width=30)
      entry.pack(anchor=tk.W)
      button = ttk.Button(self.framemain, text="Browse", command=lambda: self.filedialog_clicked(10))
      button.pack(anchor=tk.W)
      self.file_references.append((label, entry, button))
    else:
        self.DataFilepaths[10].set("")

  def create_cycle(self, *binaries):#make 8bit align (as multi ch)
    length = len(binaries[0])
    new_binary = []
    for i in range(length):
      for binary in binaries:
        new_binary.append(binary[i])
    return bytes(new_binary)

  def validate_text_entries(self):
    self.Text = []
    for text_entry in self.text_entries:
      entry_text = text_entry.get("1.0", tk.END).strip()
      if not all(0x20 <= ord(char) <= 0x7E for char in entry_text):
        tk.messagebox.showerror("Error", "Text entries must contain only ASCII characters in the range 0x20 to 0x7E.")
        return None
      self.Text.append(entry_text)
    return self.Text

  def validate_datapaths(self):
    Datapath = []
    for i, var in enumerate(self.checkbox):
      if var.get() == 1 and not self.DataFilepaths[i].get():
        tk.messagebox.showerror("Error", f"Audio data of {self.options[i]} is empty.")
        return None
    if self.channel_var.get() == 'Yes' and not self.DataFilepaths[10].get():
      tk.messagebox.showerror("Error", f"Audio data of LFE is empty.")
      return None
    for i, var in enumerate(self.DataFilepaths):
      if self.DataFilepaths[i].get():
        Datapath.append(self.DataFilepaths[i].get())
    return Datapath

  def validate_fs(self):
    if not self.Fs.get().isdigit():
      tk.messagebox.showerror("Error", "Sampling rate must be a numerical value.")
      return False
    return True
  
  def validate_checkboxes(self):
    Ch_N = int(self.Ch_N.get())
    if self.channel_var.get() == 'Yes' :
      LFE = 1
    else:
      LFE = 0
    checkbox_vars=[]
    for i, num in enumerate(self.checkbox):
      checkbox_vars.append(num.get())
    checked_count = sum(checkbox_vars)
    if (Ch_N == 1 and LFE == 0 and checked_count in [0, 2]):
      return True
    elif Ch_N == (checked_count + LFE):
      return True
    else:
      tk.messagebox.showerror("Error", "The number of channels does not match Channel Assignment.")
      return False
    

  def create_text_binary(self):
    for i, (_, max_length) in enumerate(self.fields):
      if i < len(self.Text):
        label_text = self.Text[i]
      else:
        label_text = ""
      entry_text = label_text[:max_length].ljust(max_length, ' ')
      entry_binary = entry_text.encode('utf-8').hex()
      self.Text_binary += bytes.fromhex(entry_binary)

  def read_binary_files(self, Datapath):
    binaries = []
    for path in Datapath:
      try:
        with open(path, "rb") as file:
          binaries.append(file.read())
      except Exception as e:
        tk.messagebox.showerror("Error", f"Failed to read file {path}: {e}")
        return None
    if not all(len(binary) == len(binaries[0]) for binary in binaries):
      tk.messagebox.showerror("Error", "All binary files must have the same length.")
      return None
    return binaries

  def create_cycle_wrapper(self, *binaries):
    try:
      return self.create_cycle(*binaries)
    except Exception as e:
      tk.messagebox.showerror("Error", f"Failed to create cycle: {e}")
      return None

  def conduct_main(self): # 実行時処理
    self.Text = self.validate_text_entries()
    self.create_text_binary()
    if not self.validate_checkboxes():#チャンネル数とチャンネル配置の数が一致しているか
      return
    if self.Text is None:
      return
    Datapath = self.validate_datapaths()
    if Datapath is None:#ファイルパスが指定されているか
      return
    if not self.validate_fs():#有効な標本化周波数が指定されているか
      return
    binaries = self.read_binary_files(Datapath)
    if binaries is None:
      return
    self.Stream_Data = self.create_cycle_wrapper(*binaries)#8bitアライン時のエラー
    if self.Stream_Data is None:#
      return
    self.create_wsd_file()

  def update_height(self, event, width, max_length):#テキストフィールドの高さ変更
    lines = event.widget.get("1.0", "end-1c").split("\n")
    num_lines = sum(1 + len(line) // width for line in lines)
    num_lines = min(num_lines, max_length // width)
    event.widget.config(height=num_lines)

  def enforce_max_length(self, event, max_length):#最大文字数制限
    if len(event.widget.get("1.0", "end-1c")) > max_length - 1:
      event.widget.delete("end-2c")

  def create_text_with_dynamic_height(self, parent, width, max_length):#テキストフィールドの高さ制限
    text = tk.Text(parent, width=width, height=1)
    text.bind("<Key>", lambda e: self.enforce_max_length(e, max_length))
    text.bind("<KeyRelease>", lambda e: self.update_height(e, width, max_length))
    return text

  def on_combobox_select(self, event):#選択肢に応じて入力可能とするプルダウン
    selected_value = self.Fs.get()
    if selected_value == "Other":
      self.Fs.config(state="normal")
    else:
      self.Fs.config(state="readonly")




  def create_general_information(self):
    General_Information = [ord(char) & 0xFF for char in '1bit'] # FileID (Fixed)
    General_Information += [0] * 4 # Reserved*1
    General_Information += [0X11] # Version_N (Fixed)
    General_Information += [0] * 1 # Reserved*1
    General_Information += [0] * 2 # Reserved*1
    FileSZ = 2048 + len(self.Stream_Data) # File_SZ
    FileSZ = FileSZ.to_bytes(8, 'big')
    General_Information += FileSZ[4:] + FileSZ[:4]
    General_Information += bytes.fromhex("00000080") # TextSP (Fixed)
    General_Information += bytes.fromhex("00000800") # DataSP (Fixed)
    General_Information += [0] * 4 # Reserved*1
    return General_Information

  def create_data_spec_information(self):
    # PB_TM
    tm = len(self.Stream_Data) * 8 / int(self.Fs.get()) / 3600
    PB_h = int(tm)
    PB_m = int(60 * (tm - PB_h))
    PB_s = int(60 * (60 * (tm - PB_h) - PB_m))
    Data_Spec_Information = [0, PB_s, PB_m, PB_h] #PB_TM
    Data_Spec_Information += int(self.Fs.get()).to_bytes(4, 'big') # Fs
    Data_Spec_Information += [0] * 4 # Reserved*1
    Data_Spec_Information += [int(self.Ch_N.get())] # Ch_N
    Data_Spec_Information += [0] * 3 # Reserved*1

    # Ch_Asn binary
    if self.channel_var.get() == 'Yes':
      Ch_Asn_F = f"0{''.join(str(x.get()) for x in self.checkbox[:5])}01"
    else:
      Ch_Asn_F = f"0{''.join(str(x.get()) for x in self.checkbox[:5])}00"
    Data_Spec_Information += int(Ch_Asn_F, 2).to_bytes(1, 'big') # Ch_Asn_Front
    Data_Spec_Information += [0] * 2 # Ch_Asn_Reversed
    Ch_Asn_R = Ch_Asn_F = f"0{''.join(str(x.get()) for x in self.checkbox[-5:])}01"
    Data_Spec_Information += int(Ch_Asn_R, 2).to_bytes(1, 'big') # Ch_Asn_Rear
    Data_Spec_Information += [0] * 12 # Reserved*1
    Data_Spec_Information += [0] * 4 # Emph
    Data_Spec_Information += [0] * 4 # Reserved*1
    Data_Spec_Information += [0] * 16 # TimeReference
    Data_Spec_Information += [0] * 40 # Reserved*1
    return Data_Spec_Information

  def create_text_data(self):
    Text_Data = self.Text_binary
    Text_Data += bytes([0x20] * 160)
    return Text_Data

  def create_wsd_file(self):
    General_Information = self.create_general_information()
    Data_Spec_Information = self.create_data_spec_information()
    Text_Data = self.create_text_data()
    file_data = bytes(General_Information + Data_Spec_Information)+ Text_Data + self.Stream_Data
    self.save_fire(file_data)
  
  def save_fire(self,file_data):
    save_path = filedialog.asksaveasfilename(defaultextension=".wsd",title = "Save file")
    if save_path:
      try:
        with open(save_path, 'wb') as combined_file:
          combined_file.write(file_data)
        tk.messagebox.showinfo("Success", "Created successfully.")
      except Exception as e:
        self.show_error_message(f"An error occurred: {str(e)}")


if __name__ == "__main__":
  root = tk.Tk()
  app = WSDConverter(root)
  root.mainloop()