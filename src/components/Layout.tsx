import Sidebar from "./Sidebar";
import PlayerBar from "./PlayerBar";
import { ReactNode } from "react";

interface LayoutProps {
  children: ReactNode;
}

export default function Layout({ children }: LayoutProps) {
  return (
    <div className="flex flex-col h-full w-full bg-[#0a0a0a] text-white overflow-hidden">
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <div className="flex-1 flex flex-col overflow-hidden">{children}</div>
      </div>
      <PlayerBar />
    </div>
  );
}
