import { useState } from "react";
import { Routes, Route } from "react-router";
import { AgentProvider, useAgent } from "@/lib/agent-context";
import { AgentSheet } from "@/components/AgentSheet";
import { NavChrome } from "@/components/NavChrome";
import { Onboarding } from "@/components/Onboarding";
import { Splash } from "@/components/Splash";
import Home from "@/pages/Home";
import HealthCard from "@/pages/HealthCard";
import Saved from "@/pages/Saved";
import Compare from "@/pages/Compare";
import More from "@/pages/More";

function Shell() {
  const { openAgent } = useAgent();
  return (
    <>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/building/:bbl" element={<HealthCard />} />
        <Route path="/saved" element={<Saved />} />
        <Route path="/compare" element={<Compare />} />
        <Route path="/more" element={<More />} />
        <Route path="*" element={<Home />} />
      </Routes>
      <NavChrome onOpenAgent={openAgent} />
      <AgentSheet />
    </>
  );
}

export default function App() {
  const [splashed, setSplashed] = useState(false);
  return (
    <AgentProvider>
      {!splashed && <Splash onDone={() => setSplashed(true)} />}
      <Shell />
      {/* First-launch priority picker — sits above the app, never blocks search */}
      {splashed && <Onboarding />}
    </AgentProvider>
  );
}
