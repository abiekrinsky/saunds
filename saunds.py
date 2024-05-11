import os
import random
import threading
from pydub import AudioSegment, playback
import librosa

def clean_title(filename):
    return filename.replace("_vocals", "").replace("_no_vocals", "").replace(".mp3", "").replace("_", " ").replace("_split_by_lalalai_preview", "")

def match_files(files):
    paired_files = {}
    for file in files:
        # Normalize the file name by removing common suffixes and prefixes.
        base_name = file.replace(".mp3", "").replace("_split_by_lalalai_preview", "")
        # Simplified version without replacing "_vocals_" or "_no_vocals_" yet.
        
        # Using split to isolate the main part of the filename without vocals/no_vocals tags.
        parts = base_name.split('_')
        clean_name = '_'.join([part for part in parts if part not in ['vocals', 'no', 'vocals']])
        
        # Ensure dictionary setup for tracking vocals and no_vocals
        if clean_name not in paired_files:
            paired_files[clean_name] = {"vocals": None, "no_vocals": None}
        
        # Check explicitly for "no_vocals" before "vocals" to avoid misclassification
        if "_no_vocals" in file:
            paired_files[clean_name]["no_vocals"] = file
        elif "vocals" in file and "_no_vocals" not in file:
            paired_files[clean_name]["vocals"] = file

    return paired_files

def estimate_bpm(file_path):
    try:
        y, sr = librosa.load(file_path, sr=None)
        tempo, _ = librosa.beat.beat_track(y=y, sr=sr)
        return tempo
    except Exception as e:
        print(f"Error estimating BPM for {file_path}: {e}")
        return 120  # Default BPM if analysis fails

def categorize_files(directory):
    all_files = [f for f in os.listdir(directory) if f.endswith('.mp3')]
    paired_files = match_files(all_files)
    for title, files in paired_files.items():
        if files["vocals"]:
            vocals_path = os.path.join(directory, files["vocals"])
            files["vocals_bpm"] = estimate_bpm(vocals_path)
            print(f"Vocals for {title}: {files['vocals_bpm']} BPM")
        if files["no_vocals"]:
            no_vocals_path = os.path.join(directory, files["no_vocals"])
            files["no_vocals_bpm"] = estimate_bpm(no_vocals_path)
            print(f"No Vocals for {title}: {files['no_vocals_bpm']} BPM")
    return paired_files

class AudioPlayer:
    def __init__(self, track_path, track_type, bpm, player_id):
        self.track = AudioSegment.from_file(track_path)
        self.track_type = track_type
        self.bpm = bpm
        self.player_id = player_id
        self.is_playing = False

    def play(self, speed_change=1.0):
        if not self.is_playing:
            self.is_playing = True
            # Playback without speed adjustment by default
            if speed_change != 1.0:
                playback_track = self.track._spawn(self.track.raw_data, overrides={
                    "frame_rate": int(self.track.frame_rate * speed_change)
                }).set_frame_rate(self.track.frame_rate)
            else:
                playback_track = self.track
            new_duration = int(1000 * len(self.track) / speed_change)

            print(f"Player {self.player_id} playing {self.track_type} at {speed_change:.2f}x speed. Total play time: {new_duration / 1000:.2f} seconds.")
            threading.Thread(target=playback.play, args=(playback_track,)).start()

    def stop(self):
        if self.is_playing:
            self.is_playing = False
            print(f"Player {self.player_id} stopped.")

class SplitAudioEditor:
    def __init__(self, directory):
        self.players = []
        self.directory = directory
        self.load_tracks()

    def load_tracks(self):
        pairs = self.get_randomized_pairs()
        if len(pairs) >= 2:
            self.players.append(AudioPlayer(os.path.join(self.directory, pairs[0]['vocals']), 'Vocals', pairs[0]['vocals_bpm'], 1))
            self.players.append(AudioPlayer(os.path.join(self.directory, pairs[1]['no_vocals']), 'No Vocals', pairs[1]['no_vocals_bpm'], 2))

    def get_randomized_pairs(self):
        file_pairs = categorize_files(self.directory)
        pairs_list = list(file_pairs.values())
        random.shuffle(pairs_list)
        return pairs_list

    def play_all(self):
        for player in self.players:
            player.play()  # Play at original speed by default

    def stop_all(self):
        for player in self.players:
            player.stop()

# Example usage
directory = '/Users/-/Downloads/saunds'
editor = SplitAudioEditor(directory)
editor.play_all()  # Start playback at original speeds
