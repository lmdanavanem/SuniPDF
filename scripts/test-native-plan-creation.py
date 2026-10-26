#!/usr/bin/env python3
"""Offline tests for native creation evidence helpers; no desktop qualification."""
import base64
import ast
import copy
import csv
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import re
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import MagicMock


spec = importlib.util.spec_from_file_location("native_creation", Path(__file__).with_name("native-plan-creation-fixture.py"))
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)


def creation_helper(name, namespace):
    """Execute one actual helper without starting the native app or a process."""
    path = Path(__file__).with_name("test-deb.py")
    tree = ast.parse(path.read_text(encoding="utf-8"))
    creation = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "plan_link_creation")
    helper = next(node for node in creation.body if isinstance(node, ast.FunctionDef) and node.name == name)
    exec(compile(ast.Module(body=[helper], type_ignores=[]), str(path), "exec"), namespace)
    return namespace[name]


class NativeCommandPaletteTests(unittest.TestCase):
    PREPARATIONS = ("full-rgb-nearest-2x-psm11", "full-rgb-lanczos-3x-psm11",
                    "full-rgb-lanczos-4x-psm11", "full-rgb-lanczos-4x-psm6")

    def test_protected_json_paths_are_hashed_and_changed_or_missing_inputs_are_detected(self):
        script = Path(__file__).with_name("test-deb.py")
        helper = next(node for node in ast.parse(script.read_text(encoding="utf-8")).body
                      if isinstance(node, ast.FunctionDef) and node.name == "sha")
        namespace = {"Path": Path, "hashlib": hashlib}
        exec(compile(ast.Module(body=[helper], type_ignores=[]), str(script), "exec"), namespace)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "source.pdf"
            original = b"original synthetic input"
            path.write_bytes(original)
            expected = hashlib.sha256(original).hexdigest()
            protected = json.loads(json.dumps({str(path): expected}))
            self.assertEqual(namespace["sha"](path), expected)
            self.assertTrue(all(namespace["sha"](name) == value for name, value in protected.items()))
            path.write_bytes(b"changed synthetic input")
            self.assertFalse(all(namespace["sha"](name) == value for name, value in protected.items()))
            path.unlink()
            with self.assertRaises(FileNotFoundError):
                namespace["sha"](str(path))

    def observation(self, rows, preparation=None, scale=2):
        return {"tsv": tsv(rows), "text": " ".join(words for _, words in rows),
                "scale": scale, "size": (900, 600), "ink": 0,
                "ocrPreparation": preparation or self.PREPARATIONS[0]}

    def locator(self, observed):
        capture = MagicMock(return_value="unchanged-full-frame.png")
        read_capture = MagicMock(side_effect=observed)
        namespace = {"fixtures": native, "read_capture": read_capture, "capture": capture,
                     "time": SimpleNamespace(sleep=lambda seconds: None),
                     "scroll": MagicMock(side_effect=AssertionError("Unexpected scroll"))}
        creation_helper("observations", namespace)
        return creation_helper("locate", namespace), capture, read_capture

    def test_preparations_keep_full_capture_and_distinct_immutable_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "observed.png"
            original = b"offline capture stand-in; no recognition is claimed"
            path.write_bytes(original)
            source = MagicMock()
            source.__enter__.return_value = source
            source.size = (320, 180)
            source.convert.return_value = source
            prepared = MagicMock()
            source.resize.return_value = prepared
            prepared.save.side_effect = lambda stream, format: stream.write(b"offline resized image stand-in")
            image_api = SimpleNamespace(open=MagicMock(return_value=source), NEAREST="nearest", LANCZOS="lanczos")
            command = MagicMock(return_value=tsv([(90, "Plan links")]))
            read_capture = creation_helper("read_capture", {
                "Image": image_api, "evidence": root, "command": command,
                "csv": csv, "io": io, "sha": native.sha,
            })
            files = set()
            for tag, scale, resample, psm in zip(self.PREPARATIONS, (2, 3, 4, 4),
                                                ("nearest", "lanczos", "lanczos", "lanczos"), (11, 11, 11, 6)):
                source.reset_mock()
                observed = read_capture(path, preparation=tag)
                self.assertEqual(observed["ocrPreparation"], tag)
                self.assertEqual(observed["scale"], scale)
                self.assertEqual(observed["text"], "Plan links")
                source.resize.assert_called_once_with((320 * scale, 180 * scale), resample=resample)
                source.crop.assert_not_called()
                self.assertEqual(command.call_args.args[0][-3:], ["--psm", str(psm), "tsv"])
                self.assertEqual(command.call_args.kwargs, {"timeout": 25})
                for field in ("png", "ocrInput", "ocr"):
                    self.assertEqual(observed[field + "Sha256"], native.sha(root / observed[field]))
                self.assertNotIn(observed["ocrInput"], files)
                files.add(observed["ocrInput"])
                self.assertEqual(path.read_bytes(), original)
            before = command.call_count
            retained = {name: (root / name).read_bytes() for name in files}
            with self.assertRaises(FileExistsError):
                read_capture(path)
            self.assertEqual(command.call_count, before)
            self.assertEqual(retained, {name: (root / name).read_bytes() for name in files})
            for color, tag in (("blue", self.PREPARATIONS[0]), (None, "color-mask-nearest-3x-psm11"), (None, "unlisted")):
                with self.subTest(color=color, tag=tag), self.assertRaises(AssertionError):
                    read_capture(path, color, preparation=tag)

    def test_colored_pdf_alternatives_use_only_the_same_full_frame_mask(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "pdf.png"
            path.write_bytes(b"offline PDF capture stand-in")
            source = MagicMock()
            source.__enter__.return_value = source
            source.size = (2, 2)
            source.convert.return_value.getdata.return_value = [(0, 0, 255)] * 4
            mask_image, prepared = MagicMock(), MagicMock()
            mask_image.resize.return_value = prepared
            prepared.save.side_effect = lambda stream, format: stream.write(b"offline mask image")
            image_api = SimpleNamespace(open=MagicMock(return_value=source), frombytes=MagicMock(return_value=mask_image),
                                        NEAREST="nearest", LANCZOS="lanczos")
            command, mask = MagicMock(return_value=tsv([(20, "A-101")])), MagicMock(return_value=(b"0000", 4))
            namespace = {"Image": image_api, "evidence": root, "command": command, "csv": csv,
                         "io": io, "sha": native.sha, "colored_fixture_mask": mask}
            creation_helper("read_capture", namespace)
            observations = creation_helper("observations", namespace)
            result = list(observations(path, "blue"))
            self.assertEqual([item["ocrPreparation"] for item in result],
                             ["color-mask-nearest-3x-psm11", "color-mask-nearest-3x-psm6",
                              "color-mask-lanczos-3x-psm11"])
            self.assertEqual([call.args for call in mask.call_args_list], [([(0, 0, 255)] * 4, "blue")] * 3)
            self.assertEqual([call.args for call in image_api.frombytes.call_args_list], [("L", (2, 2), b"0000")] * 3)
            self.assertEqual([call.args for call in mask_image.resize.call_args_list], [((6, 6),)] * 3)
            self.assertEqual([call.kwargs for call in mask_image.resize.call_args_list],
                             [{"resample": "nearest"}, {"resample": "nearest"}, {"resample": "lanczos"}])
            self.assertEqual([call.args[0][-3:] for call in command.call_args_list],
                             [["--psm", "11", "tsv"], ["--psm", "6", "tsv"], ["--psm", "11", "tsv"]])
            self.assertEqual({item["ink"] for item in result}, {4})
            self.assertEqual(len({item["ocrInput"] for item in result}), 3)
            self.assertEqual(len({item["ocr"] for item in result}), 3)
            source.crop.assert_not_called()
            mask_image.crop.assert_not_called()

    def test_interface_alternatives_are_finite_ordered_and_lazy(self):
        reader = MagicMock(side_effect=lambda path, color, preparation: preparation)
        observations = creation_helper("observations", {"read_capture": reader})
        choices = observations("same.png")
        self.assertEqual(next(choices), self.PREPARATIONS[0])
        self.assertEqual(reader.call_count, 1)
        self.assertEqual(tuple(choices), self.PREPARATIONS[1:])
        self.assertEqual(reader.call_count, 4)
        self.assertEqual([call.args for call in reader.call_args_list], [("same.png", None)] * 4)

    def test_original_color_mask_excludes_neutral_chrome_and_the_other_pdf_color(self):
        tree = ast.parse(Path(__file__).with_name("test-deb.py").read_text(encoding="utf-8"))
        helper = next(node for node in tree.body if isinstance(node, ast.FunctionDef)
                      and node.name == "colored_fixture_mask")
        namespace = {}
        exec(compile(ast.Module(body=[helper], type_ignores=[]), "actual-color-mask", "exec"), namespace)
        pixels = [(0, 0, 255), (255, 0, 0), (15, 23, 42), (241, 245, 249), (145, 145, 145),
                  (220, 220, 255), (100, 100, 220), (220, 100, 100)]
        self.assertEqual(namespace["colored_fixture_mask"](pixels, "blue"),
                         (bytes((0, 255, 255, 255, 255, 255, 0, 255)), 2))
        self.assertEqual(namespace["colored_fixture_mask"](pixels, "red"),
                         (bytes((255, 0, 255, 255, 255, 255, 255, 0)), 2))

    def test_locate_fallback_keeps_source_and_target_in_same_preparation(self):
        missing = self.observation([(20, "Beta.pdf"), (50, "Native destination two"), (80, "Target.pdf")])
        valid = self.observation([(20, "Alpha.pdf"), (50, "Native destination two"), (80, "Beta.pdf")], self.PREPARATIONS[1], 3)
        locate, capture, reader = self.locator([missing, valid])
        observed, box = locate("Native destination two", below_label="Alpha.pdf", above_label="Beta.pdf", allow_scroll=False)
        self.assertIs(observed, valid)
        self.assertEqual(box[1], 50)
        capture.assert_called_once_with(None)
        self.assertEqual(reader.call_count, 2)
        self.assertEqual(reader.call_args_list[0].args[0], reader.call_args_list[1].args[0])

    def test_ambiguity_is_fatal_without_trying_a_more_convenient_preparation(self):
        ambiguous = self.observation([(20, "Save"), (50, "Save")])
        valid = self.observation([(20, "Save")])
        locate, capture, reader = self.locator([ambiguous, valid])
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            locate("Save", allow_scroll=False)
        capture.assert_called_once()
        reader.assert_called_once()

    def test_colored_control_ambiguity_is_fatal_before_any_alternative(self):
        ambiguous = self.observation([(20, "A-101"), (50, "A-101")])
        locate, capture, reader = self.locator([ambiguous, self.observation([(20, "A-101")])])
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            locate("A-101", color="blue", allow_scroll=False)
        capture.assert_called_once()
        reader.assert_called_once_with("unchanged-full-frame.png", "blue",
                                       preparation="color-mask-nearest-3x-psm11")

    def test_missing_labels_cannot_borrow_anchors_or_fuzzy_names(self):
        first = self.observation([(20, "Alpha.pdf 0 Approved links"), (80, "Beta.pdf 0 Approved links")])
        second = self.observation([(20, "Alpha.pdff 0 Approved links"), (50, "A-101 Page 1"),
                                   (80, "Beta.pdf 0 Approved links")])
        locate, capture, reader = self.locator([first, second, first, second] * 6)
        with self.assertRaises(native.LabelMissing):
            locate("A-101", line_contains="Page 1", below_label=("Alpha.pdf", "0 Approved links"),
                   above_label=("Beta.pdf", "0 Approved links"), allow_scroll=False)
        self.assertEqual(capture.call_count, 6)
        self.assertEqual(reader.call_count, 24)

    def test_approval_both_bounds_exclude_the_next_source(self):
        wrong = self.observation([(20, "Alpha.pdf 0 Approved links"), (80, "Beta.pdf 0 Approved links"),
                                 (110, "A-101 Page 1")])
        valid = self.observation([(20, "Alpha.pdf 0 Approved links"), (50, "A-101 Page 1"),
                                 (80, "Beta.pdf 0 Approved links"), (110, "A-101 Page 1")])
        locate, _, reader = self.locator([wrong, valid])
        _, box = locate("A-101", line_contains="Page 1", below_label=("Alpha.pdf", "0 Approved links"),
                        above_label=("Beta.pdf", "0 Approved links"), allow_scroll=False)
        self.assertEqual(box[1], 50)
        self.assertEqual(reader.call_count, 2)

    def test_popup_scope_is_applied_to_every_preparation(self):
        wrong = self.observation([(20, "Native destination two")])
        valid = self.observation([(20, "Native destination two"), (250, "Native destination two")])
        locate, _, reader = self.locator([wrong, valid])
        _, box = locate("Native destination two", within=(0, 100, 300, 200), allow_scroll=False)
        self.assertEqual(box[1], 250)
        self.assertEqual(reader.call_count, 2)

    def test_scene_retains_winning_preparation_with_stable_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root, rows = Path(directory), [(20, "Beta.pdf 1 Approved links")]
            retained = root / "scenes"
            retained.mkdir()
            valid = self.observation(rows, self.PREPARATIONS[2], 4)
            for field, suffix in (("png", ".png"), ("ocrInput", "-prepared.png"), ("ocr", ".tsv")):
                path = root / ("source" + suffix)
                path.write_bytes(("offline retained " + field).encode())
                valid[field], valid[field + "Sha256"] = path.name, native.sha(path)
            wrong = {**valid, "text": "Beta.pdf 1 Approved links", "tsv": tsv([(20, "Beta.pdff 1 Approved links")])}
            observed = MagicMock(return_value=iter([wrong, valid]))
            namespace = {"time": SimpleNamespace(monotonic=lambda: 0), "observations": observed,
                         "capture": MagicMock(return_value="one-frame.png"), "command": lambda args: "SuniPDF",
                         "window": "offline-window", "re": re, "scenes": [], "scene_directory": retained,
                         "evidence": root, "sha": native.sha, "fixtures": native}
            creation_helper("evidence_fields", namespace)
            scene = creation_helper("scene", namespace)
            value = scene("creation-approved-beta", pattern=r"Beta\.pdf.*1 Approved links")
            self.assertEqual(value["ocrPreparation"], self.PREPARATIONS[2])
            self.assertEqual(namespace["scenes"], [value])
            namespace["capture"].assert_called_once()
            for field, suffix in (("png", ".png"), ("ocrInput", "-ocr.png"), ("ocr", ".tsv")):
                self.assertEqual(value[field], "scenes/creation-approved-beta" + suffix if "/" in value[field] else "scenes\\creation-approved-beta" + suffix)
                self.assertEqual((root / value[field]).read_bytes(), (root / valid[field]).read_bytes())
                self.assertEqual(native.sha(root / value[field]), value[field + "Sha256"])

    def test_pdf_scene_fallback_requires_exact_masked_title_and_retains_winning_tag(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            retained = root / "scenes"
            retained.mkdir()
            methods = ("color-mask-nearest-3x-psm11", "color-mask-nearest-3x-psm6",
                       "color-mask-lanczos-3x-psm11")
            wrong = self.observation([(20, "Native source bata")], methods[0], 3)
            valid = self.observation([(20, "Native source beta")], methods[1], 3)
            for observed in (wrong, valid):
                observed["ink"] = 1200
                for field, suffix in (("png", ".png"), ("ocrInput", "-prepared.png"), ("ocr", ".tsv")):
                    path = root / (observed["ocrPreparation"] + suffix)
                    path.write_bytes(("offline masked observation " + field).encode())
                    observed[field], observed[field + "Sha256"] = path.name, native.sha(path)
            reader = MagicMock(side_effect=[wrong, valid])
            namespace = {"time": SimpleNamespace(monotonic=lambda: 0), "read_capture": reader,
                         "capture": MagicMock(return_value="one-unchanged-frame.png"),
                         "command": lambda args: "SuniPDF", "window": "offline-window", "re": re,
                         "scenes": [], "scene_directory": retained, "evidence": root,
                         "sha": native.sha, "fixtures": native}
            creation_helper("observations", namespace)
            creation_helper("evidence_fields", namespace)
            scene = creation_helper("scene", namespace)
            result = scene("created-preview-beta", kind="beta")
            self.assertEqual(result["ocrContent"], "pdf-fixture")
            self.assertEqual(result["ocrPreparation"], methods[1])
            self.assertEqual(result["text"], "Native source beta")
            self.assertEqual(reader.call_count, 2)
            self.assertEqual([call.args for call in reader.call_args_list],
                             [("one-unchanged-frame.png", "blue")] * 2)
            self.assertEqual([call.kwargs["preparation"] for call in reader.call_args_list], list(methods[:2]))
            for field in ("png", "ocrInput", "ocr"):
                self.assertEqual((root / result[field]).read_bytes(), (root / valid[field]).read_bytes())
            for text in ("Native source bata", "Native source alpha", "Native destination two",
                         "Native source beta Native source beta", "Native source beta Native source alpha",
                         "Native source beta Native wrong first page"):
                self.assertFalse(native.fixture_visible("beta", 1200, text))
            self.assertFalse(native.fixture_visible("beta", 0, "Native source beta"))

    def test_command_keeps_native_keyboard_and_requires_observed_result_before_enter(self):
        calls = []
        helper = creation_helper("open_command", {
            "key": lambda *keys: calls.append(("key", keys)), "time": SimpleNamespace(sleep=lambda seconds: None),
            "command": lambda arguments: calls.append(("command", arguments)),
            "scene": lambda name, **options: calls.append(("scene", name, options)), "re": re,
        })
        helper("Plan links", "creation-command-open")
        self.assertEqual(calls, [("key", ("ctrl+k",)),
                                ("command", ["xdotool", "type", "--clearmodifiers", "--delay", "35", "Plan links"]),
                                ("scene", "creation-command-open", {"pattern": re.escape("Plan links")}),
                                ("key", ("Return",))])
        calls.clear()
        helper.__globals__["scene"] = MagicMock(side_effect=AssertionError("Not observed"))
        with self.assertRaisesRegex(AssertionError, "Not observed"):
            helper("Plan links", "creation-command-reopen")
        self.assertNotIn(("key", ("Return",)), calls)

    def test_verified_counter_keeps_qualifiers_and_never_generates_input(self):
        observed = self.observation([(110, "Destination sheets 1")])
        locate = MagicMock(return_value=(observed, (10, 110, 140, 124)))
        namespace = {"locate": locate, "actions": []}
        creation_helper("evidence_fields", namespace)
        verify = creation_helper("verify_control", namespace)
        verify("Destination sheets 1", below_label="Beta.pdf", above_label="Target.pdf")
        action = namespace["actions"][0]
        self.assertEqual(action["observedControl"], "Destination sheets 1")
        self.assertEqual((action["belowLabel"], action["aboveLabel"]), ("Beta.pdf", "Target.pdf"))
        self.assertIsNone(action["lineContains"])
        self.assertEqual(action["ocrPreparation"], self.PREPARATIONS[0])
        self.assertNotIn("click", action)

    def test_click_processes_observed_pointer_before_button_event(self):
        observed = self.observation([(110, "Choose a destination")])
        events, actions = [], []
        namespace = {"locate": MagicMock(return_value=(observed, (100, 110, 180, 130))),
                     "actions": actions, "window": "native-window",
                     "command": lambda arguments: events.append(("command", arguments)),
                     "time": SimpleNamespace(sleep=lambda seconds: events.append(("sleep", seconds)))}
        creation_helper("evidence_fields", namespace)
        click = creation_helper("click", namespace)
        self.assertEqual(click("Choose a destination", line_contains="Beta.pdf"), [70, 60])
        self.assertEqual(events, [
            ("command", ["xdotool", "mousemove", "--window", "native-window", "70", "60"]),
            ("sleep", 0.15), ("command", ["xdotool", "click", "1"]), ("sleep", 0.2)])
        self.assertEqual(actions[0]["lineContains"], "Beta.pdf")
        self.assertEqual(actions[0]["point"], [70, 60])
        self.assertEqual(actions[0]["ocrPreparation"], self.PREPARATIONS[0])

    def test_missing_popup_preserves_last_full_frame_for_every_preparation(self):
        observations = [self.observation([(100, "Beta.pdf Choose a destination")], tag)
                        for _ in range(6) for tag in self.PREPARATIONS]
        locate, capture, reader = self.locator(observations)
        with self.assertRaises(native.LabelMissing) as raised:
            locate("Native destination two", allow_scroll=False, within=(0, 0, 450, 200))
        self.assertEqual(raised.exception.observations, observations[-4:])
        self.assertEqual(capture.call_count, 6)
        self.assertEqual(reader.call_count, 24)

    def test_popup_retries_only_the_same_observed_combo_and_records_missing_frames(self):
        missing = native.LabelMissing("Option absent")
        missing.observations = [self.observation([(100, "Beta.pdf Choose a destination")], tag)
                                for tag in self.PREPARATIONS]
        calls, actions = [], []
        option_attempts = 0
        def click(label, **options):
            nonlocal option_attempts
            calls.append((label, options))
            if label == "Choose a destination":
                return (552, 581)
            option_attempts += 1
            if option_attempts == 1:
                raise missing
            return (609, 619)
        namespace = {"click": click, "fixtures": native, "actions": actions}
        creation_helper("evidence_fields", namespace)
        choose = creation_helper("choose_popup", namespace)
        self.assertEqual(choose("Choose a destination", "Native destination two", line_contains="Beta.pdf",
                                margins=(180, 120, 360, 120)), (609, 619))
        self.assertEqual(calls, [
            ("Choose a destination", {"line_contains": "Beta.pdf"}),
            ("Native destination two", {"allow_scroll": False, "within": (372, 461, 912, 701), "exact_line": False})] * 2)
        self.assertEqual(len(actions), 1)
        self.assertEqual(actions[0], {"popupRetry": "Choose a destination", "option": "Native destination two",
                                     "lineContains": "Beta.pdf", "attempt": 1, "nextAttempt": 2,
                                     "within": (372, 461, 912, 701), "exactLine": False,
                                     "observations": [namespace["evidence_fields"](item) for item in missing.observations]})
        self.assertNotIn("key", actions[0])

    def test_popup_attempts_are_finite_and_ambiguity_never_retries(self):
        for failure, expected_clicks, expected_retries in (
                (native.LabelMissing("Still absent"), 6, 2), (AssertionError("Ambiguous"), 2, 0)):
            failure.observations = [self.observation([(100, "Choose a destination")], tag)
                                    for tag in self.PREPARATIONS]
            click = MagicMock(side_effect=lambda label, **options: (552, 581) if label == "Choose a destination"
                             else (_ for _ in ()).throw(failure))
            namespace = {"click": click, "fixtures": native, "actions": []}
            creation_helper("evidence_fields", namespace)
            choose = creation_helper("choose_popup", namespace)
            with self.subTest(failure=str(failure)), self.assertRaises(type(failure)):
                choose("Choose a destination", "Native destination two", line_contains="Beta.pdf",
                       margins=(180, 120, 360, 120))
            self.assertEqual(click.call_count, expected_clicks)
            self.assertEqual(len(namespace["actions"]), expected_retries)
        namespace = {"click": MagicMock(side_effect=native.LabelMissing("Source combo absent")),
                     "fixtures": native, "actions": []}
        choose = creation_helper("choose_popup", namespace)
        with self.assertRaisesRegex(native.LabelMissing, "Source combo"):
            choose("Choose a destination", "Native destination two", line_contains="Beta.pdf",
                   margins=(180, 120, 360, 120))
        namespace["click"].assert_called_once()
        self.assertEqual(namespace["actions"], [])

    def test_preview_destination_popup_is_bound_to_observed_combo_in_both_directions(self):
        calls = []
        def click(label, **options):
            calls.append((label, options))
            return (552, 581) if label == "Choose a destination" else (180, 581)
        namespace = {"click": click, "scene": MagicMock(), "fixtures": native, "actions": []}
        creation_helper("choose_popup", namespace)
        helper = creation_helper("preview_sources", namespace)
        helper()
        choices = [options for label, options in calls if label == "Native destination two"]
        self.assertEqual(choices, [{"allow_scroll": False, "within": (372, 461, 912, 701), "exact_line": False}] * 2)
        self.assertEqual([options["line_contains"] for label, options in calls if label == "Choose a destination"],
                         ["Alpha.pdf", "Beta.pdf"])
        self.assertEqual([options for label, options in calls if label == "Beta.pdf"],
                         [{"allow_scroll": False, "within": (100, 461, 400, 701), "exact_line": True}])
        self.assertEqual(namespace["actions"], [])


class NativeSaveChooserTests(unittest.TestCase):
    @staticmethod
    def properties(pid=123, state="Normal", wm_class='"portal", "Portal"'):
        return (f'WM_STATE(WM_STATE):\n\twindow state: {state}\n\ticon window: 0x0\n'
                f'_NET_WM_PID(CARDINAL) = {pid}\nWM_CLASS(STRING) = {wm_class}\n'
                '_NET_WM_WINDOW_TYPE(ATOM) = _NET_WM_WINDOW_TYPE_DIALOG\n'
                'WM_TRANSIENT_FOR:  not found.')

    def harness(self, path, *, candidates=(202,), states=None, recognized=True,
                fallback=False, replace_identity=False, additional_after_selection=False):
        state = {"opened": False, "saved": False, "clock": 0}
        commands, captures, inputs, actions = [], [], [], []
        discovery = {"method": "EWMH _NET_CLIENT_LIST", "snapshots": []}

        def command(arguments):
            commands.append(arguments)
            if arguments == ["xprop", "-root", "_NET_CLIENT_LIST"]:
                windows = [101] + (list(candidates) if state["opened"] and not state["saved"] else [])
                if additional_after_selection and len(discovery["snapshots"]) >= 2:
                    windows.append(404)
                return '_NET_CLIENT_LIST(WINDOW): window id # ' + ', '.join(hex(value) for value in windows)
            if arguments[:2] == ["xprop", "-id"]:
                candidate = int(arguments[2])
                pid = 999 if replace_identity and len(discovery["snapshots"]) >= 3 else 123
                return self.properties(pid, (states or {}).get(candidate, "Normal"))
            if arguments[:2] == ["xdotool", "getwindowgeometry"]:
                return f'WINDOW={arguments[-1]}\nWIDTH=600\nHEIGHT=400\nX=30\nY=40\nSCREEN=0'
            self.fail(f"Unexpected native process in offline test: {arguments}")

        def save():
            path.write_text('{"offline": true}', encoding="utf-8")
            state["saved"] = True

        def click(label, **options):
            inputs.append(("click", label, options))
            if label == "Export save receipt":
                state["opened"] = True
            elif label == "Save":
                save()

        def key(value, **options):
            inputs.append(("key", value, options))
            if value == "Return" and not fallback:
                save()

        def monotonic():
            state["clock"] += 5
            return state["clock"]

        def capture(candidate):
            captures.append(candidate)
            return f'capture-{candidate}.png'

        def observations(filename):
            yield {"text": "Save Name Location" if recognized else "An unrelated application",
                   "png": filename, "pngSha256": "a" * 64, "ocr": filename + ".tsv",
                   "ocrSha256": "b" * 64, "ocrInput": filename + "-ocr.png",
                   "ocrInputSha256": "c" * 64, "ocrPreparation": "full-rgb-nearest-2x-psm11"}

        namespace = {"command": command, "re": re, "chooser_discovery": discovery,
                     "click": click, "key": key, "capture": capture, "observations": observations,
                     "actions": actions, "time": SimpleNamespace(monotonic=monotonic, sleep=lambda _: None),
                     "type_text": lambda value, **options: inputs.append(("type", str(value), options)),
                     "scene": MagicMock()}
        creation_helper("evidence_fields", namespace)
        creation_helper("managed_clients", namespace)
        export = creation_helper("export_receipt", namespace)
        return export, discovery, commands, captures, inputs, actions, namespace

    def test_frame_and_client_appearance_only_enumerates_real_managed_client(self):
        # X11 frame 303 and client 202 can display the same chooser. EWMH lists
        # only 202; a recursive search would incorrectly return both.
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            export, discovery, commands, captures, inputs, actions, _ = self.harness(path)
            export(path)
            self.assertEqual(captures, ["202"])
            self.assertFalse(any("search" in command for command in commands))
            self.assertFalse(any("303" in command for command in commands))
            self.assertEqual(discovery["selectedWindow"], "202")
            self.assertEqual(discovery["selectionSnapshot"], 1)
            self.assertEqual([row["stage"] for row in discovery["snapshots"]],
                             ["before-export", "chooser-poll", "before-location", "before-path",
                              "before-submit", "after-submit", "after-save"])
            poll = discovery["snapshots"][1]
            self.assertEqual(poll["matches"], ["202"])
            self.assertEqual(poll["clients"][1]["observations"][0]["png"], "capture-202.png")
            self.assertIn('window id # 0x65, 0xca', poll["clientList"])
            self.assertEqual(actions[0]["managedClient"],
                             {"window": "202", "pid": 123, "wmClass": '"portal", "Portal"'})
            self.assertEqual([row[1] for row in inputs if row[0] == "key"], ["ctrl+l", "Return"])
            self.assertEqual(discovery["snapshots"][-1]["clients"][0]["window"], "101")
            self.assertTrue(path.is_file())

    def test_two_real_chooser_clients_are_retained_and_refused_before_input(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            export, discovery, _, captures, inputs, actions, _ = self.harness(path, candidates=(202, 404))
            with self.assertRaisesRegex(AssertionError, "More than one"):
                export(path)
            self.assertEqual(captures, ["202", "404"])
            self.assertEqual(discovery["snapshots"][-1]["matches"], ["202", "404"])
            self.assertEqual([row[0] for row in inputs], ["click"])
            self.assertEqual(actions, [])
            self.assertFalse(path.exists())

    def test_no_new_client_unmapped_client_and_unrecognized_client_never_get_input(self):
        for options in ({"candidates": ()}, {"states": {202: "Iconic"}}, {"recognized": False}):
            with self.subTest(options=options), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "receipt.json"
                export, discovery, _, _, inputs, actions, _ = self.harness(path, **options)
                with self.assertRaisesRegex(AssertionError, "not observed"):
                    export(path)
                self.assertNotIn("selectedWindow", discovery)
                self.assertEqual([row[0] for row in inputs], ["click"])
                self.assertEqual(actions, [])
                self.assertFalse(path.exists())

    def test_changed_process_identity_refuses_before_typing(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            export, discovery, _, _, inputs, _, _ = self.harness(path, replace_identity=True)
            with self.assertRaisesRegex(AssertionError, "identity changed"):
                export(path)
            self.assertEqual(discovery["snapshots"][-1]["stage"], "before-location")
            self.assertEqual([row[0] for row in inputs], ["click"])
            self.assertFalse(path.exists())

    def test_optional_save_button_uses_same_verified_real_chooser(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            export, discovery, _, _, inputs, _, _ = self.harness(path, fallback=True)
            export(path)
            self.assertEqual(inputs[-1], ("click", "Save", {"exact_line": True, "target": "202", "allow_scroll": False}))
            self.assertEqual([row["stage"] for row in discovery["snapshots"]][-3:],
                             ["after-submit", "before-save", "after-save"])
            self.assertTrue(path.is_file())

    def test_new_managed_client_after_selection_refuses_before_typing(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            export, discovery, _, _, inputs, _, _ = self.harness(path, additional_after_selection=True)
            with self.assertRaisesRegex(AssertionError, "clients changed"):
                export(path)
            self.assertEqual([row["window"] for row in discovery["snapshots"][-1]["clients"]],
                             ["101", "202", "404"])
            self.assertEqual([row[0] for row in inputs], ["click"])
            self.assertFalse(path.exists())

    def test_malformed_root_window_identity_is_retained_and_refused(self):
        for raw in ('_NET_CLIENT_LIST: no such atom on any window.',
                    '_NET_CLIENT_LIST(WINDOW): window id # 0x0',
                    '_NET_CLIENT_LIST(WINDOW): window id # 202',
                    '_NET_CLIENT_LIST(WINDOW): window id # 0x100000000',
                    '_NET_CLIENT_LIST(WINDOW): window id # 0xa, 0xA',
                    '_NET_CLIENT_LIST(WINDOW): window id # 0xca, invalid'):
            discovery = {"snapshots": []}
            command = MagicMock(return_value=raw)
            helper = creation_helper("managed_clients", {"command": command, "re": re, "chooser_discovery": discovery})
            with self.subTest(raw=raw), self.assertRaises(AssertionError):
                helper("before-export")
            self.assertEqual(discovery["snapshots"][0]["clientList"], raw)
            command.assert_called_once_with(["xprop", "-root", "_NET_CLIENT_LIST"])

    def test_empty_client_list_is_valid_but_bad_client_properties_are_refused(self):
        discovery = {"snapshots": []}
        helper = creation_helper("managed_clients", {"command": MagicMock(return_value='_NET_CLIENT_LIST(WINDOW): window id #'),
                                                      "re": re, "chooser_discovery": discovery})
        self.assertEqual(helper("before-export")["clients"], [])
        for raw in (self.properties(pid=0), self.properties(pid=-1), self.properties(pid=4294967296),
                    self.properties(state="Withdrawn"), self.properties(wm_class='""'),
                    self.properties() + '\n_NET_WM_PID(CARDINAL) = 124'):
            discovery = {"snapshots": []}
            command = MagicMock(side_effect=['_NET_CLIENT_LIST(WINDOW): window id # 0xca', raw])
            helper = creation_helper("managed_clients", {"command": command, "re": re, "chooser_discovery": discovery})
            with self.subTest(raw=raw), self.assertRaises(AssertionError):
                helper("chooser-poll")
            self.assertEqual(discovery["snapshots"][0]["clients"][0]["properties"], raw)


def qpdf_document(link=None):
    page = {"/Type": "/Page", "/Parent": "2 0 R", "/MediaBox": [0, 0, 600, 800],
            "/Resources": {"/Font": {"/F1": "3 0 R"}}, "/Contents": "5 0 R"}
    objects = {"obj:3 0 R": {"value": {"/Type": "/Font", "/Subtype": "/Type1", "/BaseFont": "/Helvetica"}},
               "obj:4 0 R": {"value": page},
               "obj:5 0 R": {"stream": {"dict": {"/Length": 17},
                                          "data": base64.b64encode(b"synthetic content").decode()}}}
    if link is not None:
        page["/Annots"] = ["8 0 R"]
        objects["obj:8 0 R"] = {"value": link}
    return {"pages": [{"object": "4 0 R", "contents": ["5 0 R"]}], "qpdf": [{"jsonversion": 2}, objects]}


def annotation(target, name="synthetic-link-alpha"):
    return {"/Subtype": "/Link", "/P": "4 0 R", "/NM": "u:" + name, "/Rect": [72, 590, 172, 617],
            "/A": {"/S": "/GoToR", "/D": [1, "/Fit"], "/NewWindow": True,
                    "/F": {"/Type": "/Filespec", "/F": "u:Target.pdf", "/UF": "u:Target.pdf"},
                    "/SuniPDFPlanTarget": "u:" + json.dumps(target)}}


def tsv(lines):
    result = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n"
    for block, (y, words) in enumerate(lines, 1):
        x = 10
        for number, word in enumerate(words.split(), 1):
            result += f"5\t1\t{block}\t1\t1\t{number}\t{x}\t{y}\t{len(word) * 7}\t14\t95\t{word}\n"
            x += len(word) * 7 + 8
    return result


class NativeCreationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        sources = [self.root / "Alpha.pdf", self.root / "Beta.pdf"]
        outputs = [self.root / "Alpha-linked.pdf", self.root / "Beta-linked.pdf"]
        target, manifest = self.root / "Target.pdf", self.root / "plan-set.json"
        for path in [*sources, target, manifest, *outputs]:
            path.write_bytes(("offline helper fixture: " + path.name).encode())
        self.fixture = {"sources": sources, "outputs": outputs, "target": target, "manifest": manifest,
                        "sourceIds": ["source-alpha", "source-beta"], "sheetId": "target-two",
                        "protected": {str(path): native.sha(path) for path in [*sources, target, manifest]}}
        self.target = native.expected_target(self.fixture)

    def outputs(self):
        documents = {path: qpdf_document() for path in self.fixture["sources"]}
        documents.update({path: qpdf_document(annotation(self.target, "synthetic-link-" + str(index)))
                          for index, path in enumerate(self.fixture["outputs"])})
        calls = []

        def command(args):
            calls.append(args)
            self.assertEqual(args[0], "qpdf")
            return "no syntax errors" if args[1] == "--check" else json.dumps(documents[args[-1]])

        result = native.verify_outputs(self.fixture, self.root, command)
        self.assertEqual(len(calls), 8)
        return result

    def receipt(self, outputs):
        result = {"kind": "batch", "receipt": {"version": 1, "status": "complete", "failure": None,
                  "remaining": [], "last_input_validation": {"valid": True, "reason": None}, "completed": []}}
        for output in outputs:
            occurrence = {"id": "occurrence-" + output["sourceFileId"], "text": "A-101", "status": "ready",
                          "target_sheet_ids": [self.fixture["sheetId"]]}
            report = {"version": 1, "output_sha256": output["outputSha256"], "excluded_occurrences": [],
                      "inspection": {"manifest_sha256": native.sha(self.fixture["manifest"]),
                                     "source_sha256": output["sourceSha256"], "source_file_id": output["sourceFileId"],
                                     "requested_pages": [1], "scanned_pages": 1, "scan_complete": True,
                                     "truncated": False, "issues": [], "occurrences": [occurrence]},
                      "created": [{"occurrence_id": occurrence["id"], "page": 1, "annotation_index": 0,
                                   "name": output["annotationName"], "rect": output["rect"], "target": self.target}]}
            result["receipt"]["completed"].append({"output": output["output"], "bytes": output["bytes"], "report": report})
        return result

    def test_preparation_only_indexes_originals_and_never_creates_links(self):
        cli = self.root / "cli"
        cli.write_bytes(b"\x7fELF\x02\x01" + bytes(12) + b"\x3e\x00")
        calls = []

        def command(args):
            calls.append([str(item) for item in args])
            self.assertEqual(args[1], "plan-set")
            output = Path(args[-1])
            if args[2] == "create":
                document = {"files": [{"path": Path(path).name, "id": Path(path).stem} for path in args[3:6]],
                            "sheets": [{"file_id": "Target", "page": 2, "id": "sheet-two"}]}
            else:
                self.assertEqual(args[2], "edit-sheet")
                document = json.loads(Path(args[3]).read_text())
            output.write_text(json.dumps(document), encoding="utf-8")
            return "{}"

        fixture = native.prepare(self.root / "prepared", self.root, command, cli)
        self.assertEqual([call[2] for call in calls], ["create", "edit-sheet"])
        self.assertFalse(any(output.exists() for output in fixture["outputs"]))
        self.assertTrue(all(b"/Annots" not in source.read_bytes() and b"/GoToR" not in source.read_bytes()
                            for source in fixture["sources"]))
        report = json.loads((self.root / fixture["fixtureReport"]).read_text())
        self.assertFalse(report["desktopLinkSavingTested"])
        self.assertEqual(report["cliBinarySha256"], native.sha(cli))
        self.assertEqual(report["target"]["physicalPage"], 2)

    def test_ocr_scope_ignores_punctuation_but_not_ambiguous_rows(self):
        text = tsv([(20, "Alpha.pdf Â· 0 Approved links"), (50, "Beta.pdf Â· 0 Approved links"),
                    (80, "Alpha.pdf 1 Preview source Choose a destination Preview destination")])
        box = native.label_box(text, "Alpha.pdf", (900, 200), line_contains="Preview source")
        self.assertEqual(box[1], 80)
        self.assertEqual(native.label_box(text, "Beta.pdf", (900, 200), line_contains="0 Approved links")[1], 50)
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            native.label_box(text, "Alpha.pdf", (900, 200))
        with self.assertRaises(native.LabelMissing):
            native.label_box(text, "Alpha.pdf", (900, 200), line_contains="1 Approved links")
        with self.assertRaises(AssertionError):
            native.label_box(text, "Beta.pdf", (20, 200))

    def test_ocr_scopes_adjacent_sparse_blocks_to_the_same_baseline_only(self):
        text = tsv([(20, "Alpha.pdf"), (20, "Preview source"), (50, "Beta.pdf"), (80, "Preview source")])
        self.assertEqual(native.label_box(text, "Alpha.pdf", (900, 200), line_contains="Preview source")[1], 20)
        with self.assertRaises(native.LabelMissing):
            native.label_box(text, "Beta.pdf", (900, 200), line_contains="Preview source")

    def test_approval_is_bound_to_its_source_and_its_own_count(self):
        text = tsv([(20, "Alpha.pdf 1 Approved links"), (50, "A-101 Page 1"),
                    (80, "Beta.pdf 0 Approved links"), (110, "A-101 Page 1")])
        beta = native.label_box(text, "Beta.pdf", (900, 200), line_contains="0 Approved links")
        self.assertEqual(native.label_box(text, "A-101", (900, 200),
                                         line_contains="Page 1", below=beta[3])[1], 110)
        with self.assertRaises(native.LabelMissing):
            native.label_box(text, "Beta.pdf", (900, 200), line_contains="1 Approved links")
        wrong_only = tsv([(20, "Alpha.pdf 1 Approved links"), (50, "A-101 Page 1"),
                          (80, "Beta.pdf 0 Approved links")])
        with self.assertRaises(native.LabelMissing):
            native.label_box(wrong_only, "A-101", (900, 200), line_contains="Page 1", below=beta[3])

    def test_ocr_expanded_destination_group_is_below_its_source(self):
        text = tsv([(20, "Destination sheets (1)"), (50, "Beta.pdf"), (80, "Destination sheets (1)")])
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            native.label_box(text, "Destination sheets 1", (900, 200))
        self.assertEqual(native.label_box(text, "Destination sheets 1", (900, 200), below=64)[1], 80)

    def test_destination_scope_refuses_other_groups_even_when_only_wrong_label_is_recognized(self):
        text = tsv([(20, "Alpha.pdf"), (50, "Native destination two"), (80, "Beta.pdf"),
                    (110, "Native destination two"), (140, "Target.pdf")])
        self.assertEqual(native.label_box(text, "Native destination two", (900, 200), below=34, above=80)[1], 50)
        self.assertEqual(native.label_box(text, "Native destination two", (900, 200), below=94, above=140)[1], 110)
        wrong_only = tsv([(20, "Alpha.pdf"), (50, "Native destination two"), (80, "Beta.pdf"),
                          (110, "Native destination bwo"), (140, "Target.pdf")])
        with self.assertRaises(native.LabelMissing):
            native.label_box(wrong_only, "Native destination two", (900, 200), below=94, above=140)
        with self.assertRaises(native.LabelMissing):
            native.label_box(text, "Native destination two", (900, 200), below=94, above=110)

    def test_popup_choice_is_scoped_to_the_observed_combo_below_other_file_labels(self):
        text = tsv([(20, "Beta.pdf"), (80, "Beta.pdf")])
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            native.label_box(text, "Beta.pdf", (900, 200), exact_line=True)
        self.assertEqual(native.label_box(text, "Beta.pdf", (900, 200), exact_line=True,
                                         within=(0, 60, 200, 120))[1], 80)
        self.assertEqual(native.label_box(text, "Beta.pdf", (900, 200), exact_line=True,
                                         within=(0, 0, 200, 60))[1], 20)
        with self.assertRaisesRegex(AssertionError, "Ambiguous"):
            native.label_box(text, "Beta.pdf", (900, 200), exact_line=True, within=(0, 0, 200, 120))

    def test_pdf_ink_requires_unique_correct_title_and_enough_coloured_pixels(self):
        for kind, title in (("alpha", "Native source alpha"), ("beta", "Native source beta"),
                            ("destination", "Native destination two")):
            self.assertTrue(native.fixture_visible(kind, 1001, title))
            for text, count in ((title, 1000), ("Preview destination", 9000), (title + " " + title, 9000),
                                (title + " Native wrong first page", 9000), ("Other PDF", 9000)):
                with self.subTest(kind=kind, text=text):
                    self.assertFalse(native.fixture_visible(kind, count, text))

    def test_qpdf_checks_content_resources_destination_and_actual_rectangle(self):
        original = native.pdf_snapshot(qpdf_document())
        saved = qpdf_document(annotation(self.target))
        verified = native.verify_link_page(original, native.pdf_snapshot(saved), self.target)
        self.assertEqual(verified["target"]["page"], 1)
        mutations = [lambda doc: doc["qpdf"][1]["obj:5 0 R"]["stream"].update(data=base64.b64encode(b"changed").decode()),
                     lambda doc: doc["qpdf"][1]["obj:3 0 R"]["value"].update({"/BaseFont": "/Courier"}),
                     lambda doc: doc["qpdf"][1]["obj:4 0 R"]["value"].update({"/MediaBox": [0, 0, 800, 600]}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"]["/A"].update({"/D": [0, "/Fit"]}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"]["/A"].update({"/JS": "u:script"}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"].update({"/Rect": [72, 500, 172, 527]}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"].update({"/Rect": [72, 590, float("nan"), 617]}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"]["/A"]["/F"].update({"/F": "u:Other.pdf"}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"].update({"/P": "6 0 R"}),
                     lambda doc: doc["qpdf"][1]["obj:8 0 R"]["value"].update({"/AA": {}})]
        for mutate in mutations:
            changed = copy.deepcopy(saved)
            mutate(changed)
            with self.subTest(mutation=mutate), self.assertRaises(AssertionError):
                native.verify_link_page(original, native.pdf_snapshot(changed), self.target)

    def test_qpdf_rejects_old_links_and_stale_target_metadata(self):
        saved = native.pdf_snapshot(qpdf_document(annotation(self.target)))
        with self.assertRaisesRegex(AssertionError, "already contains"):
            native.verify_link_page(saved, saved, self.target)
        wrong = dict(self.target, sha256="0" * 64)
        with self.assertRaises(AssertionError):
            native.verify_link_page(native.pdf_snapshot(qpdf_document()), saved, wrong)

    def test_actual_files_and_native_receipt_are_bound_to_independent_objects(self):
        outputs = self.outputs()
        receipt = self.root / "receipt.json"
        receipt.write_text(json.dumps(self.receipt(outputs)), encoding="utf-8")
        result = native.verify_receipt(self.fixture, receipt, outputs)
        self.assertEqual(result["outputs"], 2)
        self.assertEqual(result["sha256"], native.sha(receipt))
        self.fixture["outputs"][1].write_bytes(b"replaced after saving")
        with self.assertRaises(AssertionError):
            native.verify_receipt(self.fixture, receipt, outputs)

    def test_receipt_rejects_missing_copy_wrong_hash_page_identity_or_incomplete_outcome(self):
        outputs = self.outputs()
        base = self.receipt(outputs)
        mutations = [lambda value: value["receipt"]["completed"].pop(),
                     lambda value: value["receipt"].update(status="failed"),
                     lambda value: value["receipt"].update(last_input_validation={"valid": False, "reason": "changed"}),
                     lambda value: value["receipt"]["completed"][0]["report"].update(output_sha256="0" * 64),
                     lambda value: value["receipt"]["completed"][0]["report"]["created"][0].update(page=2),
                     lambda value: value["receipt"]["completed"][0]["report"]["created"][0].update(name="another-name"),
                     lambda value: value["receipt"]["completed"][0]["report"]["inspection"].update(source_file_id="wrong"),
                     lambda value: value["receipt"]["completed"][0]["report"]["inspection"].update(manifest_sha256="0" * 64)]
        path = self.root / "receipt.json"
        for mutate in mutations:
            value = copy.deepcopy(base)
            mutate(value)
            path.write_text(json.dumps(value), encoding="utf-8")
            with self.subTest(mutation=mutate), self.assertRaises(AssertionError):
                native.verify_receipt(self.fixture, path, outputs)

    def test_changed_original_refuses_output_qualification(self):
        self.fixture["sources"][1].write_bytes(b"changed original")
        with self.assertRaisesRegex(AssertionError, "Original inputs changed"):
            native.verify_outputs(self.fixture, self.root, lambda args: self.fail("QPDF must not run for changed inputs"))


if __name__ == "__main__":
    unittest.main()
