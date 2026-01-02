import {
  BroomIcon,
  CheckIcon,
  FolderIcon,
  LinkIcon,
  TrashIcon,
} from "@phosphor-icons/react";
import { app } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useRef, useState } from "react";
import { useInView } from "react-intersection-observer";
import { commands, events, Item, SystemData } from "./bindings";

const DAYS_TO_CLEAN = 7;

const formatBytes = (bytes: number) => {
  if (bytes < 1024) {
    return `${bytes} bytes`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(2)} KB`;
  }
  if (bytes < 1024 * 1024 * 1024) {
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  }

  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
};

const isLink = (text: string) => {
  try {
    new URL(text);
    return true;
  } catch (err) {
    return false;
  }
};

function App() {
  const [history, setHistory] = useState<Item[]>([]);
  const [systemData, setSystemData] = useState<SystemData | null>(null);
  const [search, setSearch] = useState<string>("");
  const [copiedTimestamp, setCopiedTimestamp] = useState<bigint | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<bigint | null>(null);
  const [confirmCleanOldItems, setConfirmCleanOldItems] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const getHistory = async () => {
    const items = await commands.getClipboardHistory();
    setHistory(items);
  };

  const getSystemData = async () => {
    const result = await commands.getSystemData();
    setSystemData(result);
  };

  const refreshData = async () => {
    await getHistory();
    await getSystemData();
  };

  const clearClipboard = async () => {
    if (confirmClear) {
      await commands.clearClipboard();
      setConfirmClear(false);
    } else {
      setConfirmClear(true);
      setTimeout(() => {
        setConfirmClear(false);
      }, 3000);
    }
  };

  const cleanOldItems = async () => {
    if (confirmCleanOldItems) {
      await commands.cleanOldItems(DAYS_TO_CLEAN);
      setConfirmCleanOldItems(false);
    } else {
      setConfirmCleanOldItems(true);
      setTimeout(() => {
        setConfirmCleanOldItems(false);
      }, 3000);
    }
  };

  useEffect(() => {
    refreshData();

    events.clipboardChangedEvent.listen(() => {
      refreshData();
    });

    listen("tauri://focus", () => {
      inputRef.current?.focus();
      scrollRef.current?.scrollTo({
        top: 0,
        behavior: "instant",
      });
    });

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        app.hide();
      }
    };

    window.addEventListener("keydown", handleEscape);

    return () => {
      window.removeEventListener("keydown", handleEscape);
    };
  }, []);

  const handleCopy = async (item: Item) => {
    try {
      const updatedTimestamp = await commands.copy(item.timestamp);

      setCopiedTimestamp(updatedTimestamp);
      setTimeout(() => {
        setCopiedTimestamp(null);
      }, 3000);
      scrollRef.current?.scrollTo({
        top: 0,
        behavior: "smooth",
      });
    } catch (err) {
      console.error("error copying to clipboard");
      console.error(err);
    }
  };

  // Requires accessibility permission to work
  // const copyAndPaste = async (item: Item) => {
  //   try {
  //     await handleCopy(item);

  //     await invoke("paste");

  //     setCopiedTimestamp(item.timestamp);
  //     setTimeout(() => {
  //       setCopiedTimestamp(null);
  //     }, 1000);
  //   } catch (err) {
  //     console.error("error copying to clipboard");
  //     console.error(err);
  //   }
  // };

  const handleDelete = async (item: Item) => {
    if (confirmDelete === item.timestamp) {
      await commands.deleteClipboardItem(item.timestamp);
      setConfirmDelete(null);
    } else {
      setConfirmDelete(item.timestamp);
      setTimeout(() => {
        setConfirmDelete(null);
      }, 3000);
    }
  };

  return (
    <main className="bg-neutral-950/95 relative rounded-lg font-mono text-neutral-50 h-screen flex flex-col gap-2 overflow-hidden py-2 px-4">
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <h2 className="font-mono font-light">Clipboard+</h2>
          {systemData ? (
            <div className="text-xs text-neutral-500 flex items-center gap-2">
              <button
                onClick={() => revealItemInDir(systemData.db_path)}
                className="hover:text-neutral-300 flex items-center gap-1"
              >
                <FolderIcon size={16} />
              </button>
              {" - "}
              <span>
                {history.length} item{history.length === 1 ? "" : "s"} (
                {formatBytes(Number(systemData.size_bytes))})
              </span>
            </div>
          ) : null}
          <div className="flex items-center gap-2">
            {!!history.length && (
              <>
                <button
                  title={`Remove items that haven't been used in the last ${DAYS_TO_CLEAN} days`}
                  onClick={cleanOldItems}
                  className="px-2 py-1 text-xs rounded-md bg-neutral-800 hover:bg-neutral-700 font-light cursor-pointer flex items-center gap-2"
                >
                  <BroomIcon size={16} />{" "}
                  {confirmCleanOldItems ? "CONFIRM" : "CLEAN"}
                </button>
                <button
                  onClick={clearClipboard}
                  className="px-2 py-1 text-xs rounded-md bg-neutral-800 hover:bg-neutral-700 font-light cursor-pointer flex items-center gap-2"
                >
                  <TrashIcon size={16} /> {confirmClear ? "CONFIRM" : "CLEAR"}
                </button>
              </>
            )}
          </div>
        </div>
        <div>
          <input
            type="text"
            className="w-full border text-sm border-neutral-800 rounded-md px-2 py-1 font-normal focus:outline-none focus:ring-1 focus:ring-neutral-300"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search"
            autoFocus
            ref={inputRef}
          />
        </div>
      </div>
      <div
        className="flex flex-col gap-2 overflow-y-auto flex-1 pr-1.5"
        ref={scrollRef}
      >
        {!history.length ? (
          <p className="text-xs text-neutral-500 flex-1 flex items-center justify-center">
            Your clipboard is empty.
          </p>
        ) : null}
        {history
          .filter((item) =>
            search
              ? item.text?.toLowerCase().includes(search.toLowerCase())
              : true
          )
          .map((item) => (
            <div
              key={item.timestamp}
              className="group transition-colors border border-neutral-800 rounded-md p-4"
            >
              <div className="flex items-center justify-between">
                <div className="text-xs text-neutral-500 font-mono">
                  {item.source_app ? `via ${item.source_app} - ` : ""}
                  {new Date(Number(item.timestamp)).toLocaleString()} -{" "}
                  {formatBytes(item.size_bytes)}
                </div>
                <div
                  className={`flex items-center gap-2 text-xs group-hover:opacity-100 group-focus-within:opacity-100 ${
                    copiedTimestamp === item.timestamp
                      ? "opacity-100"
                      : "opacity-0"
                  }`}
                >
                  <button
                    onClick={() => handleCopy(item)}
                    className="px-2 py-1 rounded-md bg-neutral-800 hover:bg-neutral-700 font-light cursor-pointer flex items-center gap-2"
                  >
                    {copiedTimestamp === item.timestamp ? (
                      <CheckIcon size={16} />
                    ) : null}
                    {copiedTimestamp === item.timestamp ? "COPIED" : "COPY"}
                  </button>
                  <button
                    onClick={() => handleDelete(item)}
                    className="px-2 py-1 rounded-md bg-red-900 hover:bg-red-800 font-light cursor-pointer"
                  >
                    {confirmDelete === item.timestamp ? "CONFIRM" : "DELETE"}
                  </button>
                </div>
              </div>
              <div className="flex items-start gap-4 mt-0.5">
                <div className="flex-1 min-w-0">
                  {item.image ? (
                    <ImageRenderer image={item.image} />
                  ) : item.text ? (
                    <TextRenderer text={item.text} />
                  ) : null}
                </div>
              </div>
            </div>
          ))}
      </div>
    </main>
  );
}

const TextRenderer = ({ text }: { text: string }) => {
  const lines = text.split("\n").length;
  const [isCollapsed, setIsCollapsed] = useState(() => lines > 6);

  if (isLink(text)) {
    return (
      <a
        href={text}
        target="_blank"
        rel="noopener noreferrer"
        className="text-xs wrap-break-word hover:underline underline-offset-4"
      >
        {text} <LinkIcon size={16} className="inline-block" />
      </a>
    );
  }
  return (
    <div className="relative">
      <div
        className={`wrap-break-word font-mono text-xs leading-relaxed whitespace-pre-line ${
          isCollapsed ? "line-clamp-6" : "line-clamp-none"
        }`}
      >
        {text}
      </div>
      {lines > 6 && (
        <div className="absolute inset-x-0 bottom-0 py-1 flex bg-linear-to-t from-neutral-950 via-neutral-950/90 to-transparent">
          <button
            onClick={() => setIsCollapsed(!isCollapsed)}
            className="text-xs uppercase font-light cursor-pointer bg-neutral-800 hover:bg-neutral-700 rounded-md px-2 py-1"
          >
            {isCollapsed ? "expand" : "collapse"}
          </button>
        </div>
      )}
    </div>
  );
};

const ImageRenderer = ({ image }: { image: string }) => {
  const { inView, ref } = useInView();

  return (
    <div ref={ref} className="max-h-[120px]">
      {inView ? (
        <img
          src={image}
          alt="Clipboard Image"
          className="object-contain max-h-[120px]"
        />
      ) : (
        <div className="bg-neutral-800 animate-pulse h-[120px]">Loading...</div>
      )}
    </div>
  );
};

export default App;
